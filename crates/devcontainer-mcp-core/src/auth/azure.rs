use std::collections::HashMap;
use std::process::Stdio;

use async_trait::async_trait;
use tokio::process::Command;

use super::{AuthAccount, AuthLoginResult, AuthProvider, AuthStatus};
use crate::cli::{run_cli, CliBinary};
use crate::error::Result;

pub struct AzureAuth;

#[async_trait]
impl AuthProvider for AzureAuth {
    fn name(&self) -> &str {
        "azure"
    }

    async fn status(&self) -> Result<AuthStatus> {
        let output = run_cli(&CliBinary::Az, &["account", "list", "-o", "json"], false).await;
        let output = match output {
            Ok(o) => o,
            Err(_) => {
                return Ok(AuthStatus {
                    provider: "azure".into(),
                    cli_installed: false,
                    accounts: vec![],
                });
            }
        };

        let mut accounts = vec![];
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&output.stdout) {
            if let Some(arr) = parsed.as_array() {
                for entry in arr {
                    let name = entry
                        .get("name")
                        .and_then(|n| n.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let sub_id = entry
                        .get("id")
                        .and_then(|i| i.as_str())
                        .unwrap_or("")
                        .to_string();
                    let user = entry
                        .get("user")
                        .and_then(|u| u.get("name"))
                        .and_then(|n| n.as_str())
                        .unwrap_or("")
                        .to_string();
                    let is_default = entry
                        .get("isDefault")
                        .and_then(|d| d.as_bool())
                        .unwrap_or(false);
                    let state = entry
                        .get("state")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();

                    accounts.push(AuthAccount {
                        id: format!("azure-{sub_id}"),
                        login: format!("{name} ({user})"),
                        active: is_default,
                        metadata: serde_json::json!({
                            "subscription_id": sub_id,
                            "subscription_name": name,
                            "user": user,
                            "state": state,
                        }),
                    });
                }
            }
        }

        Ok(AuthStatus {
            provider: "azure".into(),
            cli_installed: true,
            accounts,
        })
    }

    async fn login(&self, _scopes: Option<&str>) -> Result<AuthLoginResult> {
        let child = Command::new("az")
            .args(["login", "--use-device-code"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|_| crate::error::Error::AzCliNotFound)?;

        let output = child
            .wait_with_output()
            .await
            .map_err(crate::error::Error::Io)?;
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            let status = self.status().await?;
            let active = status.accounts.into_iter().find(|a| a.active);
            Ok(AuthLoginResult {
                id: active.map(|a| a.id),
                action: "success".into(),
                message: "Azure authentication complete.".into(),
                browser_opened: stderr.contains("open"),
                code_copied: false,
            })
        } else {
            Ok(AuthLoginResult {
                id: None,
                action: "error".into(),
                message: format!("Azure authentication failed: {}", stderr.trim()),
                browser_opened: false,
                code_copied: false,
            })
        }
    }

    async fn select(&self, handle: &str) -> Result<Option<AuthAccount>> {
        let sub_id = handle.strip_prefix("azure-").unwrap_or(handle);
        let output = run_cli(
            &CliBinary::Az,
            &["account", "set", "--subscription", sub_id],
            false,
        )
        .await?;
        if output.exit_code != 0 {
            return Ok(None);
        }
        let status = self.status().await?;
        Ok(status.accounts.into_iter().find(|a| a.active))
    }

    async fn resolve_env(&self, handle: &str) -> Result<HashMap<String, String>> {
        let sub_id = handle.strip_prefix("azure-").unwrap_or(handle);
        let mut env = HashMap::new();
        env.insert("AZURE_SUBSCRIPTION_ID".into(), sub_id.to_string());
        Ok(env)
    }

    async fn logout(&self, _handle: &str) -> Result<String> {
        let output = run_cli(&CliBinary::Az, &["logout"], false).await?;
        if output.exit_code == 0 {
            Ok("Logged out of Azure.".into())
        } else {
            Ok(format!("Azure logout failed: {}", output.stderr.trim()))
        }
    }
}
