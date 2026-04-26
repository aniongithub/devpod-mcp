use serde::{Deserialize, Serialize};

use crate::cli::{run_cli, CliBinary, CliOutput};
use crate::error::{Error, Result};

/// Run a devpod CLI command with the given args.
async fn run_devpod(args: &[&str], parse_json: bool) -> Result<CliOutput> {
    run_cli(&CliBinary::DevPod, args, parse_json).await
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
pub async fn up(args: &[&str]) -> Result<CliOutput> {
    let mut cmd_args = vec!["up", "--open-ide=false"];
    cmd_args.extend_from_slice(args);
    run_devpod(&cmd_args, false).await
}

/// `devpod stop` — stop a workspace.
pub async fn stop(workspace: &str) -> Result<CliOutput> {
    run_devpod(&["stop", workspace], false).await
}

/// `devpod delete` — delete a workspace.
pub async fn delete(workspace: &str, force: bool) -> Result<CliOutput> {
    let mut args = vec!["delete", workspace];
    if force {
        args.push("--force");
    }
    run_devpod(&args, false).await
}

/// `devpod build` — build a workspace image.
pub async fn build(args: &[&str]) -> Result<CliOutput> {
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
pub async fn status(workspace: &str, timeout: Option<&str>) -> Result<CliOutput> {
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
pub async fn list() -> Result<CliOutput> {
    run_devpod(&["list", "--output", "json"], true).await
}

// ---------------------------------------------------------------------------
// Command execution
// ---------------------------------------------------------------------------

/// `devpod ssh --command` — execute a command in a workspace.
pub async fn ssh_exec(
    workspace: &str,
    command: &str,
    user: Option<&str>,
    workdir: Option<&str>,
) -> Result<CliOutput> {
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
pub async fn logs(workspace: &str) -> Result<CliOutput> {
    run_devpod(&["logs", workspace], false).await
}

// ---------------------------------------------------------------------------
// Provider management
// ---------------------------------------------------------------------------

/// `devpod provider list` — list providers.
pub async fn provider_list() -> Result<CliOutput> {
    run_devpod(&["provider", "list", "--output", "json"], true).await
}

/// `devpod provider add` — add a provider.
pub async fn provider_add(provider: &str, options: &[&str]) -> Result<CliOutput> {
    let mut args = vec!["provider", "add", provider];
    args.extend_from_slice(options);
    run_devpod(&args, false).await
}

/// `devpod provider delete` — delete a provider.
pub async fn provider_delete(provider: &str) -> Result<CliOutput> {
    run_devpod(&["provider", "delete", provider], false).await
}

// ---------------------------------------------------------------------------
// Context management
// ---------------------------------------------------------------------------

/// `devpod context list` — list contexts.
pub async fn context_list() -> Result<CliOutput> {
    run_devpod(&["context", "list", "--output", "json"], true).await
}

/// `devpod context use` — switch context.
pub async fn context_use(context: &str) -> Result<CliOutput> {
    run_devpod(&["context", "use", context], false).await
}

// ---------------------------------------------------------------------------
// Import / Export
// ---------------------------------------------------------------------------

/// `devpod import` — import a workspace.
pub async fn import(args: &[&str]) -> Result<CliOutput> {
    let mut cmd_args = vec!["import"];
    cmd_args.extend_from_slice(args);
    run_devpod(&cmd_args, false).await
}

/// `devpod export` — export a workspace.
pub async fn export(workspace: &str) -> Result<CliOutput> {
    run_devpod(&["export", workspace], false).await
}

// ---------------------------------------------------------------------------
// File operations
// ---------------------------------------------------------------------------

/// Read a file from a DevPod workspace.
pub async fn file_read(
    workspace: &str,
    path: &str,
    user: Option<&str>,
) -> Result<CliOutput> {
    let cmd = crate::file_ops::read_file_command(path);
    ssh_exec(workspace, &cmd, user, None).await
}

/// Write (create or overwrite) a file in a DevPod workspace.
pub async fn file_write(
    workspace: &str,
    path: &str,
    content: &str,
    user: Option<&str>,
) -> Result<CliOutput> {
    let cmd = crate::file_ops::write_file_command(path, content);
    ssh_exec(workspace, &cmd, user, None).await
}

/// Surgical edit: replace exactly one occurrence of `old_str` with `new_str`.
pub async fn file_edit(
    workspace: &str,
    path: &str,
    old_str: &str,
    new_str: &str,
    user: Option<&str>,
) -> Result<String> {
    let read_output = file_read(workspace, path, user).await?;
    if read_output.exit_code != 0 {
        return Err(Error::FileRead(format!(
            "Failed to read {path}: {}",
            read_output.stderr.trim()
        )));
    }

    let modified = crate::file_ops::apply_edit(&read_output.stdout, old_str, new_str)?;

    let write_output = file_write(workspace, path, &modified, user).await?;
    if write_output.exit_code != 0 {
        return Err(Error::FileEdit(format!(
            "Failed to write {path}: {}",
            write_output.stderr.trim()
        )));
    }

    Ok(format!("Edit applied to {path}"))
}

/// List directory contents in a DevPod workspace.
pub async fn file_list(
    workspace: &str,
    path: &str,
    user: Option<&str>,
) -> Result<CliOutput> {
    let cmd = crate::file_ops::list_dir_command(path);
    ssh_exec(workspace, &cmd, user, None).await
}
