use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;

use crate::error::{Error, Result};

/// Raw output from a DevPod CLI invocation.
#[derive(Debug, Clone, Serialize)]
pub struct DevPodOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    /// Parsed JSON from stdout, if the command was invoked with --output json.
    pub json: Option<serde_json::Value>,
}

/// Run a devpod CLI command with the given args.
/// If `parse_json` is true, attempts to parse stdout as JSON.
async fn run_devpod(args: &[&str], parse_json: bool) -> Result<DevPodOutput> {
    let output = Command::new("devpod")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                Error::DevPodNotFound
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

    Ok(DevPodOutput {
        exit_code,
        stdout,
        stderr,
        json,
    })
}

/// Check that the `devpod` CLI is available on PATH.
pub async fn check_cli() -> Result<String> {
    let output = run_devpod(&["version"], false).await?;
    if output.exit_code == 0 {
        Ok(output.stdout.trim().to_string())
    } else {
        Err(Error::DevPodNotFound)
    }
}

// ---------------------------------------------------------------------------
// Workspace lifecycle
// ---------------------------------------------------------------------------

/// `devpod up` — create and start a workspace.
pub async fn up(args: &[&str]) -> Result<DevPodOutput> {
    let mut cmd_args = vec!["up", "--open-ide=false"];
    cmd_args.extend_from_slice(args);
    run_devpod(&cmd_args, false).await
}

/// `devpod stop` — stop a workspace.
pub async fn stop(workspace: &str) -> Result<DevPodOutput> {
    run_devpod(&["stop", workspace], false).await
}

/// `devpod delete` — delete a workspace.
pub async fn delete(workspace: &str, force: bool) -> Result<DevPodOutput> {
    let mut args = vec!["delete", workspace];
    if force {
        args.push("--force");
    }
    run_devpod(&args, false).await
}

/// `devpod build` — build a workspace image.
pub async fn build(args: &[&str]) -> Result<DevPodOutput> {
    let mut cmd_args = vec!["build"];
    cmd_args.extend_from_slice(args);
    run_devpod(&cmd_args, false).await
}

// ---------------------------------------------------------------------------
// Workspace queries
// ---------------------------------------------------------------------------

/// Workspace status from `devpod status --output json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStatus {
    pub id: Option<String>,
    pub context: Option<String>,
    pub provider: Option<String>,
    pub state: Option<String>,
}

/// `devpod status` — get workspace status as JSON.
pub async fn status(workspace: &str, timeout: Option<&str>) -> Result<DevPodOutput> {
    let mut args = vec!["status", workspace, "--output", "json"];
    if let Some(t) = timeout {
        args.push("--timeout");
        args.push(t);
    }
    run_devpod(&args, true).await
}

/// Workspace list entry from `devpod list --output json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceListEntry {
    #[serde(flatten)]
    pub data: serde_json::Value,
}

/// `devpod list` — list all workspaces as JSON.
pub async fn list() -> Result<DevPodOutput> {
    run_devpod(&["list", "--output", "json"], true).await
}

// ---------------------------------------------------------------------------
// Command execution
// ---------------------------------------------------------------------------

/// `devpod ssh --command` — execute a command in a workspace.
pub async fn ssh_exec(workspace: &str, command: &str, user: Option<&str>, workdir: Option<&str>) -> Result<DevPodOutput> {
    let mut args = vec!["ssh", workspace, "--command", command];
    if let Some(u) = user {
        args.push("--user");
        args.push(u);
    }
    if let Some(w) = workdir {
        args.push("--workdir");
        args.push(w);
    }
    run_devpod(&args, false).await
}

// ---------------------------------------------------------------------------
// Logs
// ---------------------------------------------------------------------------

/// `devpod logs` — get workspace logs.
pub async fn logs(workspace: &str) -> Result<DevPodOutput> {
    run_devpod(&["logs", workspace], false).await
}

// ---------------------------------------------------------------------------
// Provider management
// ---------------------------------------------------------------------------

/// `devpod provider list` — list providers.
pub async fn provider_list() -> Result<DevPodOutput> {
    run_devpod(&["provider", "list", "--output", "json"], true).await
}

/// `devpod provider add` — add a provider.
pub async fn provider_add(provider: &str, options: &[&str]) -> Result<DevPodOutput> {
    let mut args = vec!["provider", "add", provider];
    args.extend_from_slice(options);
    run_devpod(&args, false).await
}

/// `devpod provider delete` — delete a provider.
pub async fn provider_delete(provider: &str) -> Result<DevPodOutput> {
    run_devpod(&["provider", "delete", provider], false).await
}

// ---------------------------------------------------------------------------
// Context management
// ---------------------------------------------------------------------------

/// `devpod context list` — list contexts.
pub async fn context_list() -> Result<DevPodOutput> {
    run_devpod(&["context", "list", "--output", "json"], true).await
}

/// `devpod context use` — switch context.
pub async fn context_use(context: &str) -> Result<DevPodOutput> {
    run_devpod(&["context", "use", context], false).await
}

// ---------------------------------------------------------------------------
// Import / Export
// ---------------------------------------------------------------------------

/// `devpod import` — import a workspace.
pub async fn import(args: &[&str]) -> Result<DevPodOutput> {
    let mut cmd_args = vec!["import"];
    cmd_args.extend_from_slice(args);
    run_devpod(&cmd_args, false).await
}

/// `devpod export` — export a workspace.
pub async fn export(workspace: &str) -> Result<DevPodOutput> {
    run_devpod(&["export", workspace], false).await
}
