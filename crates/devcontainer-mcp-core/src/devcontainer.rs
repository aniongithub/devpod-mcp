use crate::cli::{run_cli, ChunkSink, CliBinary, CliOutput};
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
/// fires, every descendant inside the container is reaped via a docker
/// exec-side `kill -- -<pgid>` against the process group the shim
/// established. `on_chunk`, if supplied, receives every line of
/// stdout/stderr as the child emits it — typically wired to an MCP
/// progress notification on the server side.
///
/// This function differs from the generic [`crate::cli::run_cli_streaming`]
/// in one important way: container descendants are not in the host PID
/// namespace lineage of `devcontainer exec` (the docker daemon
/// reparents them under containerd-shim), so a `/proc` walk on the host
/// would miss them. We install a tiny `setsid` + sentinel shim around
/// the user command and use the captured PGID to reap them on cancel.
pub async fn exec_streaming(
    workspace_folder: &str,
    command: &str,
    command_args: &[&str],
    cancel: &CancellationToken,
    on_chunk: Option<Arc<dyn ChunkSink>>,
) -> Result<CliOutput> {
    use crate::cli::{OutputChunk, OutputStream};
    use std::process::Stdio;
    use std::sync::Mutex;
    use tokio::io::{AsyncBufReadExt, BufReader};
    use tokio::process::Command;

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
    // it inside the container.
    let remote_env_arg = format!("{}={}", crate::exec_shim::USER_CMD_ENV, user_cmd);

    // Spawn `devcontainer exec --workspace-folder <ws> --remote-env … sh -c <wrapped>`.
    let mut cmd = Command::new(crate::cli::CliBinary::Devcontainer.command_name());
    cmd.args([
        "exec",
        "--workspace-folder",
        workspace_folder,
        "--remote-env",
        &remote_env_arg,
        "sh",
        "-c",
        &wrapped,
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .kill_on_drop(true);

    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            crate::error::Error::DevcontainerCliNotFound
        } else {
            crate::error::Error::Io(e)
        }
    })?;

    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");

    // Shared captured PGID. The stderr reader fills this in as soon as
    // the shim's sentinel line arrives; the cancel branch reads it.
    let pgid: Arc<Mutex<Option<i32>>> = Arc::new(Mutex::new(None));

    // stdout: forward verbatim (no sentinel to scrub).
    let stdout_task = {
        let sink = on_chunk.clone();
        tokio::spawn(async move {
            let mut buf = String::new();
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if !buf.is_empty() {
                    buf.push('\n');
                }
                buf.push_str(&line);
                if let Some(s) = sink.as_ref() {
                    s.on_chunk(OutputChunk {
                        stream: OutputStream::Stdout,
                        line,
                    })
                    .await;
                }
            }
            if !buf.is_empty() {
                buf.push('\n');
            }
            buf
        })
    };

    // stderr: scrub the sentinel line, otherwise forward verbatim.
    let stderr_task = {
        let sink = on_chunk.clone();
        let pgid = pgid.clone();
        tokio::spawn(async move {
            let mut buf = String::new();
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(found) = crate::exec_shim::try_parse_sentinel(&line) {
                    *pgid.lock().expect("pgid lock") = Some(found);
                    tracing::debug!(pgid = found, "captured in-container PGID");
                    continue; // suppress sentinel from user-visible stderr
                }
                if !buf.is_empty() {
                    buf.push('\n');
                }
                buf.push_str(&line);
                if let Some(s) = sink.as_ref() {
                    s.on_chunk(OutputChunk {
                        stream: OutputStream::Stderr,
                        line,
                    })
                    .await;
                }
            }
            if !buf.is_empty() {
                buf.push('\n');
            }
            buf
        })
    };

    let status = tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            tracing::warn!("devcontainer exec: cancellation received");

            // Kill the in-container process group first, then the host
            // wrapper. Order matters: if we kill the host wrapper first,
            // docker exec closes its stdio and we lose the channel —
            // but the in-container processes keep running anyway.
            let captured = *pgid.lock().expect("pgid lock");
            match captured {
                Some(pgid) => {
                    if let Err(e) = kill_in_container_pgid(workspace_folder, pgid).await {
                        tracing::warn!(%e, "failed to kill in-container PGID");
                    }
                }
                None => {
                    // Sentinel never arrived — either the shim hasn't
                    // emitted yet (very fast cancel) or `setsid` isn't
                    // available and the fallback path didn't emit
                    // before we cancelled. Either way, host-side reap
                    // is our only lever.
                    tracing::warn!(
                        "no in-container PGID captured; relying on host reap (may leak)"
                    );
                }
            }

            // Now bring down the host wrapper and any of its host-side
            // descendants we *can* see.
            if let Some(root_pid) = child.id() {
                crate::process_tree::reap(root_pid, std::time::Duration::from_secs(2)).await;
            }

            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                child.wait(),
            )
            .await;
            let _ = stdout_task.await;
            let _ = stderr_task.await;
            return Err(crate::error::Error::Cancelled);
        }
        status = child.wait() => status?,
    };

    let stdout_buf = stdout_task.await.unwrap_or_default();
    let stderr_buf = stderr_task.await.unwrap_or_default();

    Ok(CliOutput {
        exit_code: status.code().unwrap_or(-1),
        stdout: stdout_buf,
        stderr: stderr_buf,
        json: None,
    })
}

/// Send SIGTERM (then SIGKILL after a 2s grace) to the in-container
/// process group `pgid`. Uses bollard's exec API so it works against the
/// docker daemon without shelling out.
async fn kill_in_container_pgid(workspace_folder: &str, pgid: i32) -> Result<()> {
    use bollard::exec::{CreateExecOptions, StartExecOptions};

    let client = docker::connect()?;
    let container = docker::find_container_by_local_folder(&client, workspace_folder)
        .await?
        .ok_or_else(|| {
            crate::error::Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("No devcontainer found for workspace: {workspace_folder}"),
            ))
        })?;

    // `kill -<sig> -<pgid>` signals every process in the group. We
    // deliberately omit the POSIX `--` argument separator because
    // BusyBox/dash `kill` (common in slim container images) rejects
    // it as "Illegal number". The form `kill -TERM -N` is accepted
    // by every shell `kill` we care about.
    let run = |sig: &'static str| -> futures_util::future::BoxFuture<'static, ()> {
        let client = client.clone();
        let container_id = container.id.clone();
        let cmd = format!("kill -{sig} -{pgid} 2>/dev/null || true");
        Box::pin(async move {
            let create = CreateExecOptions {
                cmd: Some(vec!["sh".to_string(), "-c".to_string(), cmd]),
                attach_stdout: Some(false),
                attach_stderr: Some(false),
                ..Default::default()
            };
            match client.create_exec(&container_id, create).await {
                Ok(res) => {
                    // `detach: true` makes start_exec return as soon as
                    // the docker daemon has spawned the kill — we don't
                    // need to stream its (empty) output. The signal will
                    // be delivered before we proceed to the next step.
                    let opts = StartExecOptions {
                        detach: true,
                        ..Default::default()
                    };
                    if let Err(e) = client.start_exec(&res.id, Some(opts)).await {
                        tracing::debug!(%e, "start_exec for in-container kill failed");
                    }
                }
                Err(e) => {
                    tracing::debug!(%e, "create_exec for in-container kill failed");
                }
            }
        })
    };

    tracing::debug!(pgid, "in-container SIGTERM -<pgid>");
    run("TERM").await;
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    tracing::debug!(pgid, "in-container SIGKILL -<pgid>");
    run("KILL").await;
    Ok(())
}

/// `devcontainer build` — build a dev container image.
pub async fn build(workspace_folder: &str, extra_args: &[&str]) -> Result<CliOutput> {
    let mut args = vec!["build", "--workspace-folder", workspace_folder];
    args.extend_from_slice(extra_args);
    run_devcontainer(&args, true).await
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
