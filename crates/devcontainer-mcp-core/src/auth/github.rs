use std::collections::HashMap;
use std::process::Stdio;

use async_trait::async_trait;
use tokio::process::Command;

use super::{AuthAccount, AuthLoginResult, AuthProvider, AuthStatus};
use crate::cli::{run_cli, CliBinary};
use crate::error::Result;

pub struct GitHubAuth;

#[async_trait]
impl AuthProvider for GitHubAuth {
    fn name(&self) -> &str {
        "github"
    }

    async fn status(&self) -> Result<AuthStatus> {
        // Check if gh is installed
        let output = run_cli(
            &CliBinary::Gh,
            &["auth", "status", "--json", "hosts"],
            false,
        )
        .await;
        let output = match output {
            Ok(o) => o,
            Err(_) => {
                return Ok(AuthStatus {
                    provider: "github".into(),
                    cli_installed: false,
                    accounts: vec![],
                });
            }
        };

        let mut accounts = vec![];
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
            if let Some(hosts) = parsed.get("hosts").and_then(|h| h.get("github.com")) {
                if let Some(arr) = hosts.as_array() {
                    for entry in arr {
                        if entry.get("state").and_then(|s| s.as_str()) == Some("success") {
                            let login = entry
                                .get("login")
                                .and_then(|l| l.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let active = entry
                                .get("active")
                                .and_then(|a| a.as_bool())
                                .unwrap_or(false);
                            let scopes = entry
                                .get("scopes")
                                .and_then(|s| s.as_str())
                                .unwrap_or("")
                                .to_string();

                            accounts.push(AuthAccount {
                                id: format!("github-{login}"),
                                login,
                                active,
                                metadata: serde_json::json!({ "scopes": scopes }),
                            });
                        }
                    }
                }
            }
        }

        Ok(AuthStatus {
            provider: "github".into(),
            cli_installed: true,
            accounts,
        })
    }

    async fn login(&self, scopes: Option<&str>) -> Result<AuthLoginResult> {
        let mut args = vec!["auth", "login", "-h", "github.com", "-p", "https", "-w"];
        let scope_str;
        if let Some(s) = scopes {
            scope_str = s.to_string();
            args.push("-s");
            args.push(&scope_str);
        }

        // Spawn the login process and read its output for the device code
        let child = Command::new("gh")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| crate::error::Error::GhCliNotFound)?;

        let output = child
            .wait_with_output()
            .await
            .map_err(crate::error::Error::Io)?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("{stdout}{stderr}");

        if output.status.success() {
            // Try to figure out which account was authenticated
            let status = self.status().await?;
            let active = status.accounts.into_iter().find(|a| a.active);
            let id = active.as_ref().map(|a| a.id.clone());

            Ok(AuthLoginResult {
                id,
                action: "success".into(),
                message: "Authentication complete.".into(),
                browser_opened: true,
                code_copied: combined.contains("copied"),
            })
        } else {
            Ok(AuthLoginResult {
                id: None,
                action: "error".into(),
                message: format!("Authentication failed: {}", combined.trim()),
                browser_opened: false,
                code_copied: false,
            })
        }
    }

    async fn verify(&self, handle: &str) -> Result<Option<AuthAccount>> {
        let login = handle.strip_prefix("github-").unwrap_or(handle);
        let status = self.status().await?;
        Ok(status.accounts.into_iter().find(|a| a.login == login))
    }

    async fn resolve_env(&self, handle: &str) -> Result<HashMap<String, String>> {
        let login = handle.strip_prefix("github-").unwrap_or(handle);
        let output = run_cli(
            &CliBinary::Gh,
            &["auth", "token", "-h", "github.com", "--user", login],
            false,
        )
        .await?;

        let token = output.stdout.trim().to_string();
        if token.is_empty() {
            return Err(crate::error::Error::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("Could not get token for GitHub account: {login}"),
            )));
        }

        let mut env = HashMap::new();
        env.insert("GH_TOKEN".into(), token);
        Ok(env)
    }
}
