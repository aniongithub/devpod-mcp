pub mod github;

use async_trait::async_trait;
use serde::Serialize;

/// An auth account/identity discovered from a provider's CLI.
#[derive(Debug, Clone, Serialize)]
pub struct AuthAccount {
    /// Opaque handle the agent passes to tools, e.g. "github-aniongithub"
    pub id: String,
    /// Display name / login
    pub login: String,
    /// Whether this is the CLI's currently active account
    pub active: bool,
    /// Provider-specific metadata (scopes, subscription, project, etc.)
    pub metadata: serde_json::Value,
}

/// Result of checking auth status for a provider.
#[derive(Debug, Clone, Serialize)]
pub struct AuthStatus {
    pub provider: String,
    pub cli_installed: bool,
    pub accounts: Vec<AuthAccount>,
}

/// Result of an auth_login flow.
#[derive(Debug, Clone, Serialize)]
pub struct AuthLoginResult {
    /// The auth handle for the newly authenticated account, if successful.
    pub id: Option<String>,
    /// What happened: "success", "device_code", "browser", "error"
    pub action: String,
    /// Human-readable message for the agent to relay.
    pub message: String,
    /// Whether the browser was opened automatically.
    pub browser_opened: bool,
    /// Whether a device code was copied to the clipboard.
    pub code_copied: bool,
}

/// Trait implemented by each auth provider (github, aws, azure, etc.)
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// Provider name, e.g. "github", "aws"
    fn name(&self) -> &str;

    /// Check which accounts/identities are available.
    async fn status(&self) -> crate::error::Result<AuthStatus>;

    /// Initiate a login flow. May open a browser, copy device codes, etc.
    /// `scopes` is provider-specific (e.g. "codespace" for GitHub).
    async fn login(&self, scopes: Option<&str>) -> crate::error::Result<AuthLoginResult>;

    /// Verify that a handle is still valid and return its account info.
    async fn verify(&self, handle: &str) -> crate::error::Result<Option<AuthAccount>>;

    /// Resolve a handle to the environment variables needed by the subprocess.
    /// e.g. github → { "GH_TOKEN": "<token>" }
    async fn resolve_env(
        &self,
        handle: &str,
    ) -> crate::error::Result<std::collections::HashMap<String, String>>;
}

/// Get a provider by name.
pub fn get_provider(name: &str) -> Option<Box<dyn AuthProvider>> {
    match name {
        "github" => Some(Box::new(github::GitHubAuth)),
        // Future: "aws" => Some(Box::new(aws::AwsAuth)),
        // Future: "azure" => Some(Box::new(azure::AzureAuth)),
        // Future: "gcloud" => Some(Box::new(gcloud::GcloudAuth)),
        // Future: "kubernetes" => Some(Box::new(kubernetes::K8sAuth)),
        _ => None,
    }
}

/// Extract the provider name from a handle (e.g. "github-aniongithub" → "github").
pub fn provider_from_handle(handle: &str) -> Option<&str> {
    handle.split('-').next()
}

/// Resolve a handle to env vars by looking up the right provider.
pub async fn resolve_handle_env(
    handle: &str,
) -> crate::error::Result<std::collections::HashMap<String, String>> {
    let provider_name = provider_from_handle(handle).ok_or_else(|| {
        crate::error::Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Invalid auth handle: {handle}"),
        ))
    })?;
    let provider = get_provider(provider_name).ok_or_else(|| {
        crate::error::Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Unknown auth provider: {provider_name}"),
        ))
    })?;
    provider.resolve_env(handle).await
}
