use std::collections::HashMap;
use std::process::Stdio;

use async_trait::async_trait;
use tokio::process::Command;

use super::{AuthAccount, AuthLoginResult, AuthProvider, AuthStatus};
use crate::cli::{run_cli, CliBinary};
use crate::error::Result;

pub struct AwsAuth;

#[async_trait]
impl AuthProvider for AwsAuth {
    fn name(&self) -> &str {
        "aws"
    }

    async fn status(&self) -> Result<AuthStatus> {
        // Check if aws is installed by running sts get-caller-identity
        let output = run_cli(
            &CliBinary::Aws,
            &["sts", "get-caller-identity", "--output", "json"],
            false,
        )
        .await;
        let output = match output {
            Ok(o) => o,
            Err(_) => {
                return Ok(AuthStatus {
                    provider: "aws".into(),
                    cli_installed: false,
                    accounts: vec![],
                });
            }
        };

        let mut accounts = vec![];

        // If the default profile works, add it
        if output.exit_code == 0 {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                let account = parsed
                    .get("Account")
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_string();
                let arn = parsed
                    .get("Arn")
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_string();
                let user_id = parsed
                    .get("UserId")
                    .and_then(|u| u.as_str())
                    .unwrap_or("")
                    .to_string();

                accounts.push(AuthAccount {
                    id: "aws-default".into(),
                    login: arn.to_string(),
                    active: true,
                    metadata: serde_json::json!({
                        "profile": "default",
                        "account": account,
                        "user_id": user_id,
                        "arn": arn,
                    }),
                });
            }
        }

        // Try to list named profiles from config
        let profiles_output =
            run_cli(&CliBinary::Aws, &["configure", "list-profiles"], false).await;
        if let Ok(po) = profiles_output {
            if po.exit_code == 0 {
                for profile in po.stdout.lines() {
                    let profile = profile.trim();
                    if profile.is_empty() || profile == "default" {
                        continue;
                    }
                    accounts.push(AuthAccount {
                        id: format!("aws-{profile}"),
                        login: profile.to_string(),
                        active: false,
                        metadata: serde_json::json!({ "profile": profile }),
                    });
                }
            }
        }

        Ok(AuthStatus {
            provider: "aws".into(),
            cli_installed: true,
            accounts,
        })
    }

    async fn login(&self, scopes: Option<&str>) -> Result<AuthLoginResult> {
        // scopes is used as profile name for SSO login
        let profile = scopes.unwrap_or("default");
        let child = Command::new("aws")
            .args(["sso", "login", "--profile", profile])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| crate::error::Error::AwsCliNotFound)?;

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
            Ok(AuthLoginResult {
                id: Some(format!("aws-{profile}")),
                action: "success".into(),
                message: format!("AWS SSO login complete for profile '{profile}'."),
                browser_opened: combined.contains("browser"),
                code_copied: false,
            })
        } else {
            Ok(AuthLoginResult {
                id: None,
                action: "error".into(),
                message: format!("AWS login failed: {}", combined.trim()),
                browser_opened: false,
                code_copied: false,
            })
        }
    }

    async fn verify(&self, handle: &str) -> Result<Option<AuthAccount>> {
        let profile = handle.strip_prefix("aws-").unwrap_or(handle);
        let output = run_cli(
            &CliBinary::Aws,
            &[
                "sts",
                "get-caller-identity",
                "--profile",
                profile,
                "--output",
                "json",
            ],
            false,
        )
        .await?;

        if output.exit_code == 0 {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
                let arn = parsed
                    .get("Arn")
                    .and_then(|a| a.as_str())
                    .unwrap_or("")
                    .to_string();
                return Ok(Some(AuthAccount {
                    id: handle.to_string(),
                    login: arn,
                    active: profile == "default",
                    metadata: parsed,
                }));
            }
        }
        Ok(None)
    }

    async fn resolve_env(&self, handle: &str) -> Result<HashMap<String, String>> {
        let profile = handle.strip_prefix("aws-").unwrap_or(handle);
        let mut env = HashMap::new();
        env.insert("AWS_PROFILE".into(), profile.to_string());
        Ok(env)
    }
}
