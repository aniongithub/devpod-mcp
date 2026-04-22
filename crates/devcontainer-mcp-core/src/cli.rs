use serde::Serialize;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;

use crate::error::{Error, Result};

/// Raw output from a CLI invocation (shared across all backends).
#[derive(Debug, Clone, Serialize)]
pub struct CliOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    /// Parsed JSON from stdout, if applicable.
    pub json: Option<serde_json::Value>,
}

/// Which CLI binary to invoke.
pub enum CliBinary {
    DevPod,
    Devcontainer,
    /// GitHub CLI — the actual binary is `gh`.
    Gh,
    /// Azure CLI
    Az,
    /// AWS CLI
    Aws,
    /// Google Cloud CLI
    Gcloud,
    /// Kubernetes CLI
    Kubectl,
}

impl CliBinary {
    pub fn command_name(&self) -> &'static str {
        match self {
            CliBinary::DevPod => "devpod",
            CliBinary::Devcontainer => "devcontainer",
            CliBinary::Gh => "gh",
            CliBinary::Az => "az",
            CliBinary::Aws => "aws",
            CliBinary::Gcloud => "gcloud",
            CliBinary::Kubectl => "kubectl",
        }
    }

    fn not_found_error(&self) -> Error {
        match self {
            CliBinary::DevPod => Error::DevPodNotFound,
            CliBinary::Devcontainer => Error::DevcontainerCliNotFound,
            CliBinary::Gh => Error::GhCliNotFound,
            CliBinary::Az => Error::AzCliNotFound,
            CliBinary::Aws => Error::AwsCliNotFound,
            CliBinary::Gcloud => Error::GcloudCliNotFound,
            CliBinary::Kubectl => Error::KubectlNotFound,
        }
    }
}

/// Run a CLI command, capturing stdout/stderr/exit_code.
/// If `parse_json` is true, attempts to parse stdout as JSON.
pub async fn run_cli(binary: &CliBinary, args: &[&str], parse_json: bool) -> Result<CliOutput> {
    run_cli_with_env(binary, args, parse_json, None).await
}

/// Run a CLI command with optional environment variable overrides.
pub async fn run_cli_with_env(
    binary: &CliBinary,
    args: &[&str],
    parse_json: bool,
    env: Option<&HashMap<String, String>>,
) -> Result<CliOutput> {
    let mut cmd = Command::new(binary.command_name());
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

    if let Some(env_vars) = env {
        for (k, v) in env_vars {
            cmd.env(k, v);
        }
    }

    let output = cmd.output().await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            binary.not_found_error()
        } else {
            Error::Io(e)
        }
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    let json = if parse_json {
        serde_json::from_str(&stdout).ok()
    } else {
        None
    };

    Ok(CliOutput {
        exit_code,
        stdout,
        stderr,
        json,
    })
}
