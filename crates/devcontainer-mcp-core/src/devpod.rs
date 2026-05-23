use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::cli::{run_cli, run_with_shim, ChunkSink, CliBinary, CliOutput, RemoteKiller};
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

/// `devpod ssh --command` — cancellable, streaming variant.
///
/// Same shim treatment as [`crate::devcontainer::exec_streaming`]: the
/// user's command is wrapped with [`crate::exec_shim::wrap_self_contained`]
/// so it survives the SSH transport without quoting hazards, the
/// captured remote PGID is stripped from stderr, and on cancel a
/// second `devpod ssh --command` delivers `kill -TERM -<pgid>` then
/// `kill -KILL -<pgid>` to the same workspace.
///
/// DevPod (unlike the devcontainer CLI) has no `--remote-env` flag,
/// hence the self-contained shim variant which embeds the user
/// command as a base64 blob inside the shell snippet.
pub async fn ssh_exec_streaming(
    workspace: &str,
    command: &str,
    user: Option<&str>,
    workdir: Option<&str>,
    cancel: &CancellationToken,
    on_chunk: Option<Arc<dyn ChunkSink>>,
) -> Result<CliOutput> {
    let wrapped = crate::exec_shim::wrap_self_contained(command);
    let mut args = vec!["ssh", workspace, "--command", &wrapped];
    if let Some(u) = user {
        args.push("--user");
        args.push(u);
    }
    if let Some(w) = workdir {
        args.push("--workdir");
        args.push(w);
    }

    let killer: Arc<dyn RemoteKiller> = Arc::new(DevpodKiller {
        workspace: workspace.to_string(),
        user: user.map(str::to_string),
    });

    run_with_shim(
        &CliBinary::DevPod,
        &args,
        None,
        cancel,
        on_chunk,
        killer,
    )
    .await
}

/// Delivers `kill -<sig> -<pgid>` inside a DevPod workspace by
/// spawning a fresh short-lived `devpod ssh --command "kill ..."`.
///
/// We can't reuse the original SSH session (it's busy running the
/// workload that we're trying to interrupt), so each kill is its own
/// `devpod ssh` round trip. This is slower than the docker exec path
/// — SSH auth handshake plus DevPod's own session setup — but cancel
/// is a rare path and we'd rather pay 1–2 seconds of latency than
/// leak the workload.
struct DevpodKiller {
    workspace: String,
    user: Option<String>,
}

#[async_trait::async_trait]
impl RemoteKiller for DevpodKiller {
    async fn kill_pgid(&self, pgid: i32, signal: &str) {
        // `kill -<sig> -<pgid>` with the same BusyBox-friendly form
        // we use for the devcontainer backend (no `--`).
        let cmd = format!("kill -{signal} -{pgid} 2>/dev/null || true");
        let mut args = vec!["ssh", &self.workspace, "--command", &cmd];
        if let Some(u) = self.user.as_deref() {
            args.push("--user");
            args.push(u);
        }
        // Use the plain (non-streaming, non-cancellable) runner so we
        // don't recursively pull in the whole shim machinery for a
        // 50-byte one-shot. We swallow errors: a kill that fails
        // because the target has already exited is fine.
        if let Err(e) = run_cli(&CliBinary::DevPod, &args, false).await {
            tracing::debug!(%e, pgid, signal, "devpod ssh kill failed");
        }
    }
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
pub async fn file_read(workspace: &str, path: &str, user: Option<&str>) -> Result<CliOutput> {
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
pub async fn file_list(workspace: &str, path: &str, user: Option<&str>) -> Result<CliOutput> {
    let cmd = crate::file_ops::list_dir_command(path);
    ssh_exec(workspace, &cmd, user, None).await
}
