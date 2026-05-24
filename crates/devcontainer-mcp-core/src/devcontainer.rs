use crate::cli::{
    run_cli, run_cli_streaming, run_with_shim, ChunkSink, CliBinary, CliOutput, RemoteKiller,
};
use crate::docker;
use crate::error::Result;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Run a `devcontainer` CLI command.
async fn run_devcontainer(args: &[&str], parse_json: bool) -> Result<CliOutput> {
    run_cli(&CliBinary::Devcontainer, args, parse_json).await
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// `devcontainer up` — create and start a dev container.
pub async fn up(
    workspace_folder: &str,
    config: Option<&str>,
    extra_args: &[&str],
) -> Result<CliOutput> {
    let mut args = vec!["up", "--workspace-folder", workspace_folder];
    if let Some(c) = config {
        args.push("--config");
        args.push(c);
    }
    args.extend_from_slice(extra_args);
    run_devcontainer(&args, true).await
}

/// `devcontainer up` — cancellable, streaming variant.
///
/// `devcontainer up` does a docker build + container create + the
/// devcontainer lifecycle commands (postCreate, postStart, etc.) all
/// in one invocation. That can easily take several minutes on a
/// cold image. Same rationale as `crate::devpod::up_streaming`:
/// cancellation prevents leaked partial-up containers, and progress
/// streaming keeps client transports warm.
pub async fn up_streaming(
    workspace_folder: &str,
    config: Option<&str>,
    extra_args: &[&str],
    cancel: &CancellationToken,
    on_chunk: Option<Arc<dyn ChunkSink>>,
) -> Result<CliOutput> {
    let mut args = vec!["up", "--workspace-folder", workspace_folder];
    if let Some(c) = config {
        args.push("--config");
        args.push(c);
    }
    args.extend_from_slice(extra_args);
    run_cli_streaming(
        &CliBinary::Devcontainer,
        &args,
        true,
        None,
        cancel,
        on_chunk,
    )
    .await
}

/// `devcontainer exec` — execute a command in a running dev container.
pub async fn exec(
    workspace_folder: &str,
    command: &str,
    command_args: &[&str],
) -> Result<CliOutput> {
    let mut args = vec!["exec", "--workspace-folder", workspace_folder, command];
    args.extend_from_slice(command_args);
    run_devcontainer(&args, false).await
}

/// `devcontainer exec` — cancellable, streaming variant.
///
/// `cancel` is honored at any point during the child's lifetime; if it
/// fires, every descendant inside the container is reaped via a
/// bollard `create_exec` + `start_exec` of `kill -<sig> -<pgid>`
/// against the process group the shim established. `on_chunk`, if
/// supplied, receives every line of stdout/stderr as the child emits
/// it — typically wired to an MCP progress notification on the server
/// side.
///
/// Container descendants are not in the host PID namespace lineage of
/// `devcontainer exec` (the docker daemon reparents them under
/// containerd-shim), so a `/proc` walk on the host would miss them.
/// We install a tiny `setsid` + sentinel shim around the user command
/// and use the captured PGID to reap them on cancel.
pub async fn exec_streaming(
    workspace_folder: &str,
    command: &str,
    command_args: &[&str],
    cancel: &CancellationToken,
    on_chunk: Option<Arc<dyn ChunkSink>>,
) -> Result<CliOutput> {
    // Build the user-side command string. The MCP handler invokes us
    // as `("sh", &["-c", &full_cmd])`, which is the fast path: peel
    // off the `-c` arg so the shim wraps the body directly instead of
    // nesting an extra shell. For any other shape, fall back to a
    // best-effort shell-quoted concatenation.
    let user_cmd: String = if command == "sh" && command_args.len() == 2 && command_args[0] == "-c"
    {
        command_args[1].to_string()
    } else {
        let mut parts = vec![crate::file_ops::shell_quote(command)];
        for a in command_args {
            parts.push(crate::file_ops::shell_quote(a));
        }
        parts.join(" ")
    };

    let wrapped = crate::exec_shim::wrap();
    // The user command is passed to the shim via an env var so it
    // doesn't need to be quoted into a nested `sh -c '...'`. We use
    // `--remote-env NAME=VALUE` so the devcontainer CLI propagates
    // it inside the container. This is the env-var path; the
    // self-contained `wrap_self_contained` variant is for backends
    // (DevPod, Codespaces) that can't pass remote env vars.
    let remote_env_arg = format!("{}={}", crate::exec_shim::USER_CMD_ENV, user_cmd);

    let args: [&str; 7] = [
        "exec",
        "--workspace-folder",
        workspace_folder,
        "--remote-env",
        &remote_env_arg,
        "sh",
        "-c",
    ];
    // run_with_shim takes a slice of &str; build a Vec to add the wrapped tail.
    let mut all_args: Vec<&str> = args.to_vec();
    all_args.push(&wrapped);

    let killer: Arc<dyn RemoteKiller> = Arc::new(DevcontainerKiller {
        workspace_folder: workspace_folder.to_string(),
    });

    run_with_shim(
        &CliBinary::Devcontainer,
        &all_args,
        None,
        cancel,
        on_chunk,
        killer,
    )
    .await
}

/// Delivers `kill -<sig> -<pgid>` inside the devcontainer associated
/// with a workspace folder, using bollard's exec API.
struct DevcontainerKiller {
    workspace_folder: String,
}

#[async_trait::async_trait]
impl RemoteKiller for DevcontainerKiller {
    async fn kill_pgid(&self, pgid: i32, signal: &str) {
        use bollard::exec::{CreateExecOptions, StartExecOptions};

        let client = match docker::connect() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(%e, "failed to connect to docker for in-container kill");
                return;
            }
        };
        let container = match docker::find_container_by_local_folder(&client, &self.workspace_folder).await {
            Ok(Some(c)) => c,
            Ok(None) => {
                tracing::warn!(
                    workspace = %self.workspace_folder,
                    "no devcontainer found for in-container kill"
                );
                return;
            }
            Err(e) => {
                tracing::warn!(%e, "container lookup failed during in-container kill");
                return;
            }
        };

        // `kill -<sig> -<pgid>` signals every process in the group.
        // The POSIX `--` argument separator is deliberately omitted
        // because BusyBox/dash `kill` (common in slim container
        // images) rejects it as "Illegal number".
        let cmd = format!("kill -{signal} -{pgid} 2>/dev/null || true");
        let create = CreateExecOptions {
            cmd: Some(vec!["sh".to_string(), "-c".to_string(), cmd]),
            attach_stdout: Some(false),
            attach_stderr: Some(false),
            ..Default::default()
        };
        let res = match client.create_exec(&container.id, create).await {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!(%e, "create_exec for in-container kill failed");
                return;
            }
        };
        // `detach: true` makes start_exec return as soon as the docker
        // daemon has spawned the kill; we don't need to stream its
        // (empty) output.
        let opts = StartExecOptions {
            detach: true,
            ..Default::default()
        };
        if let Err(e) = client.start_exec(&res.id, Some(opts)).await {
            tracing::debug!(%e, "start_exec for in-container kill failed");
        }
    }
}

/// `devcontainer build` — build a dev container image.
pub async fn build(workspace_folder: &str, extra_args: &[&str]) -> Result<CliOutput> {
    let mut args = vec!["build", "--workspace-folder", workspace_folder];
    args.extend_from_slice(extra_args);
    run_devcontainer(&args, true).await
}

/// `devcontainer build` — cancellable, streaming variant. See
/// [`up_streaming`].
pub async fn build_streaming(
    workspace_folder: &str,
    extra_args: &[&str],
    cancel: &CancellationToken,
    on_chunk: Option<Arc<dyn ChunkSink>>,
) -> Result<CliOutput> {
    let mut args = vec!["build", "--workspace-folder", workspace_folder];
    args.extend_from_slice(extra_args);
    run_cli_streaming(
        &CliBinary::Devcontainer,
        &args,
        true,
        None,
        cancel,
        on_chunk,
    )
    .await
}

/// `devcontainer read-configuration` — read devcontainer config as JSON.
pub async fn read_configuration(workspace_folder: &str, config: Option<&str>) -> Result<CliOutput> {
    let mut args = vec!["read-configuration", "--workspace-folder", workspace_folder];
    if let Some(c) = config {
        args.push("--config");
        args.push(c);
    }
    args.push("--include-merged-configuration");
    run_devcontainer(&args, true).await
}

// ---------------------------------------------------------------------------
// Lifecycle via bollard (devcontainer CLI has no stop/down)
// ---------------------------------------------------------------------------

/// Stop a dev container found by its workspace folder label.
pub async fn stop(workspace_folder: &str) -> Result<String> {
    let client = docker::connect()?;
    let container = docker::find_container_by_local_folder(&client, workspace_folder)
        .await?
        .ok_or_else(|| {
            crate::error::Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("No devcontainer found for workspace: {workspace_folder}"),
            ))
        })?;
    docker::stop_container(&client, &container.id).await?;
    Ok(format!("Stopped container {}", container.name))
}

/// Remove a dev container found by its workspace folder label.
pub async fn remove(workspace_folder: &str, force: bool) -> Result<String> {
    let client = docker::connect()?;
    let container = docker::find_container_by_local_folder(&client, workspace_folder)
        .await?
        .ok_or_else(|| {
            crate::error::Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("No devcontainer found for workspace: {workspace_folder}"),
            ))
        })?;
    docker::remove_container(&client, &container.id, force).await?;
    Ok(format!("Removed container {}", container.name))
}

/// Get status of a dev container by workspace folder label.
pub async fn status(workspace_folder: &str) -> Result<Option<docker::ContainerInfo>> {
    let client = docker::connect()?;
    docker::find_container_by_local_folder(&client, workspace_folder).await
}

// ---------------------------------------------------------------------------
// File operations
// ---------------------------------------------------------------------------

/// Read a file from a dev container.
pub async fn file_read(workspace_folder: &str, path: &str) -> Result<CliOutput> {
    let cmd = crate::file_ops::read_file_command(path);
    exec(workspace_folder, "sh", &["-c", &cmd]).await
}

/// Write (create or overwrite) a file in a dev container.
pub async fn file_write(workspace_folder: &str, path: &str, content: &str) -> Result<CliOutput> {
    let cmd = crate::file_ops::write_file_command(path, content);
    exec(workspace_folder, "sh", &["-c", &cmd]).await
}

/// Surgical edit: replace exactly one occurrence of `old_str` with `new_str`.
pub async fn file_edit(
    workspace_folder: &str,
    path: &str,
    old_str: &str,
    new_str: &str,
) -> Result<String> {
    let read_output = file_read(workspace_folder, path).await?;
    if read_output.exit_code != 0 {
        return Err(crate::error::Error::FileRead(format!(
            "Failed to read {path}: {}",
            read_output.stderr.trim()
        )));
    }

    let modified = crate::file_ops::apply_edit(&read_output.stdout, old_str, new_str)?;

    let write_output = file_write(workspace_folder, path, &modified).await?;
    if write_output.exit_code != 0 {
        return Err(crate::error::Error::FileEdit(format!(
            "Failed to write {path}: {}",
            write_output.stderr.trim()
        )));
    }

    Ok(format!("Edit applied to {path}"))
}

/// List directory contents in a dev container.
pub async fn file_list(workspace_folder: &str, path: &str) -> Result<CliOutput> {
    let cmd = crate::file_ops::list_dir_command(path);
    exec(workspace_folder, "sh", &["-c", &cmd]).await
}
