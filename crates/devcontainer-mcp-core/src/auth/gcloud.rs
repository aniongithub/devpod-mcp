use std::collections::HashMap;
use std::process::Stdio;

use async_trait::async_trait;
use tokio::process::Command;

use super::{AuthAccount, AuthLoginResult, AuthProvider, AuthStatus};
use crate::cli::{run_cli, CliBinary};
use crate::error::Result;

pub struct GcloudAuth;

#[async_trait]
impl AuthProvider for GcloudAuth {
    fn name(&self) -> &str {
        "gcloud"
    }

    async fn status(&self) -> Result<AuthStatus> {
        let output = run_cli(
            &CliBinary::Gcloud,
            &["auth", "list", "--format=json"],
            false,
        )
        .await;
        let output = match output {
            Ok(o) => o,
            Err(_) => {
                return Ok(AuthStatus {
                    provider: "gcloud".into(),
                    cli_installed: false,
                    accounts: vec![],
                });
            }
        };

        let mut accounts = vec![];
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
            if let Some(arr) = parsed.as_array() {
                for entry in arr {
                    let account = entry
                        .get("account")
                        .and_then(|a| a.as_str())
                        .unwrap_or("")
                        .to_string();
                    let status = entry
                        .get("status")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();

                    accounts.push(AuthAccount {
                        id: format!("gcloud-{account}"),
                        login: account,
                        active: status == "ACTIVE",
                        metadata: serde_json::json!({ "status": status }),
                    });
                }
            }
        }

        Ok(AuthStatus {
            provider: "gcloud".into(),
            cli_installed: true,
            accounts,
        })
    }

    async fn login(&self, _scopes: Option<&str>) -> Result<AuthLoginResult> {
        let child = Command::new("gcloud")
            .args(["auth", "login", "--no-browser"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| crate::error::Error::GcloudCliNotFound)?;

        let output = child
            .wait_with_output()
            .await
            .map_err(crate::error::Error::Io)?;
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        if output.status.success() {
            let status = self.status().await?;
            let active = status.accounts.into_iter().find(|a| a.active);
            Ok(AuthLoginResult {
                id: active.map(|a| a.id),
                action: "success".into(),
                message: "Google Cloud authentication complete.".into(),
                browser_opened: false,
                code_copied: false,
            })
        } else {
            Ok(AuthLoginResult {
                id: None,
                action: "error".into(),
                message: format!("Google Cloud authentication failed: {}", combined.trim()),
                browser_opened: false,
                code_copied: false,
            })
        }
    }

    async fn verify(&self, handle: &str) -> Result<Option<AuthAccount>> {
        let account = handle.strip_prefix("gcloud-").unwrap_or(handle);
        let status = self.status().await?;
        Ok(status.accounts.into_iter().find(|a| a.login == account))
    }

    async fn resolve_env(&self, handle: &str) -> Result<HashMap<String, String>> {
        let account = handle.strip_prefix("gcloud-").unwrap_or(handle);
        let mut env = HashMap::new();
        env.insert("CLOUDSDK_CORE_ACCOUNT".into(), account.to_string());
        Ok(env)
    }
}
