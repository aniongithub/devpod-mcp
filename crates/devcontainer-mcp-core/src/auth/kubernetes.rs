use std::collections::HashMap;

use async_trait::async_trait;

use super::{AuthAccount, AuthLoginResult, AuthProvider, AuthStatus};
use crate::cli::{run_cli, CliBinary};
use crate::error::Result;

pub struct KubernetesAuth;

#[async_trait]
impl AuthProvider for KubernetesAuth {
    fn name(&self) -> &str {
        "kubernetes"
    }

    async fn status(&self) -> Result<AuthStatus> {
        let output = run_cli(
            &CliBinary::Kubectl,
            &["config", "get-contexts", "-o", "name"],
            false,
        )
        .await;
        let output = match output {
            Ok(o) => o,
            Err(_) => {
                return Ok(AuthStatus {
                    provider: "kubernetes".into(),
                    cli_installed: false,
                    accounts: vec![],
                });
            }
        };

        // Get current context
        let current = run_cli(&CliBinary::Kubectl, &["config", "current-context"], false)
            .await
            .ok()
            .map(|o| o.stdout.trim().to_string())
            .unwrap_or_default();

        let mut accounts = vec![];
        for line in output.stdout.lines() {
            let ctx = line.trim();
            if ctx.is_empty() {
                continue;
            }
            accounts.push(AuthAccount {
                id: format!("k8s-{ctx}"),
                login: ctx.to_string(),
                active: ctx == current,
                metadata: serde_json::json!({ "context": ctx }),
            });
        }

        Ok(AuthStatus {
            provider: "kubernetes".into(),
            cli_installed: true,
            accounts,
        })
    }

    async fn login(&self, scopes: Option<&str>) -> Result<AuthLoginResult> {
        // Kubernetes doesn't have a login command — contexts are configured
        // via cloud CLIs (gcloud, az, aws) or kubeconfig files.
        // If a context name is provided as scopes, switch to it.
        if let Some(context) = scopes {
            let output = run_cli(
                &CliBinary::Kubectl,
                &["config", "use-context", context],
                false,
            )
            .await?;
            if output.exit_code == 0 {
                return Ok(AuthLoginResult {
                    id: Some(format!("k8s-{context}")),
                    action: "success".into(),
                    message: format!("Switched to Kubernetes context '{context}'."),
                    browser_opened: false,
                    code_copied: false,
                });
            } else {
                return Ok(AuthLoginResult {
                    id: None,
                    action: "error".into(),
                    message: format!("Failed to switch context: {}", output.stderr.trim()),
                    browser_opened: false,
                    code_copied: false,
                });
            }
        }

        Ok(AuthLoginResult {
            id: None,
            action: "error".into(),
            message: "Kubernetes auth is managed via kubeconfig contexts. \
                      Use auth_status(provider: 'kubernetes') to list available contexts, \
                      then auth_login(provider: 'kubernetes', scopes: '<context-name>') to switch."
                .into(),
            browser_opened: false,
            code_copied: false,
        })
    }

    async fn select(&self, handle: &str) -> Result<Option<AuthAccount>> {
        let context = handle.strip_prefix("k8s-").unwrap_or(handle);
        let output = run_cli(
            &CliBinary::Kubectl,
            &["config", "use-context", context],
            false,
        )
        .await?;
        if output.exit_code != 0 {
            return Ok(None);
        }
        let status = self.status().await?;
        Ok(status.accounts.into_iter().find(|a| a.login == context))
    }

    async fn resolve_env(&self, handle: &str) -> Result<HashMap<String, String>> {
        let context = handle.strip_prefix("k8s-").unwrap_or(handle);
        let mut env = HashMap::new();
        env.insert("KUBECTL_CONTEXT".into(), context.to_string());
        Ok(env)
    }

    async fn logout(&self, handle: &str) -> Result<String> {
        let context = handle.strip_prefix("k8s-").unwrap_or(handle);
        let output = run_cli(
            &CliBinary::Kubectl,
            &["config", "delete-context", context],
            false,
        )
        .await?;
        if output.exit_code == 0 {
            Ok(format!("Deleted Kubernetes context: {context}"))
        } else {
            Ok(format!(
                "Failed to delete context: {}",
                output.stderr.trim()
            ))
        }
    }
}
