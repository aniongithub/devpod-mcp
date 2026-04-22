use thiserror::Error;

/// Unified error type for devcontainer-mcp-core.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Docker error: {0}")]
    Docker(#[from] bollard::errors::Error),

    #[error("DevPod CLI not found. Install from: https://devpod.sh/docs/getting-started/install")]
    DevPodNotFound,

    #[error("devcontainer CLI not found. Install with: npm install -g @devcontainers/cli")]
    DevcontainerCliNotFound,

    #[error("GitHub CLI (gh) not found. Install from: https://cli.github.com/")]
    GhCliNotFound,

    #[error("Azure CLI (az) not found. Install from: https://learn.microsoft.com/en-us/cli/azure/install-azure-cli")]
    AzCliNotFound,

    #[error("AWS CLI not found. Install from: https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html")]
    AwsCliNotFound,

    #[error("Google Cloud CLI (gcloud) not found. Install from: https://cloud.google.com/sdk/docs/install")]
    GcloudCliNotFound,

    #[error("kubectl not found. Install from: https://kubernetes.io/docs/tasks/tools/")]
    KubectlNotFound,

    #[error("DevPod command failed (exit code {exit_code}): {stderr}")]
    DevPodCommand { exit_code: i32, stderr: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
