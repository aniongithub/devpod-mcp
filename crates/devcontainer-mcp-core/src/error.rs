use thiserror::Error;

/// Unified error type for devcontainer-mcp-core.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Docker error: {0}")]
    Docker(#[from] bollard::errors::Error),

    #[error("DevPod CLI not found. Install from: https://devpod.sh/docs/getting-started/install")]
    DevPodNotFound,

    #[error("DevPod command failed (exit code {exit_code}): {stderr}")]
    DevPodCommand {
        exit_code: i32,
        stderr: String,
    },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
