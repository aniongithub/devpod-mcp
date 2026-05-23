use serde::Serialize;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

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

/// Which output stream a chunk came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// A chunk of output from a streaming CLI invocation.
#[derive(Debug, Clone)]
pub struct OutputChunk {
    pub stream: OutputStream,
    /// One line of output (without trailing newline).
    pub line: String,
}

/// Sink for streamed output chunks.
///
/// Implementations should be fast — they're awaited inline by the
/// background dispatcher and slow sinks will buffer up in the bounded
/// channel. Errors must be swallowed inside the sink (we don't want to
/// abort the underlying process because progress reporting failed).
#[async_trait::async_trait]
pub trait ChunkSink: Send + Sync + 'static {
    async fn on_chunk(&self, chunk: OutputChunk);
}

/// Run a CLI command, capturing stdout/stderr/exit_code.
/// If `parse_json` is true, attempts to parse stdout as JSON.
///
/// Convenience wrapper around [`run_cli_streaming`] with no progress sink
/// and a fresh (never-fired) cancellation token. Callers that want
/// cancellation or progress should use [`run_cli_streaming`] directly.
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
    run_cli_streaming(
        binary,
        args,
        parse_json,
        env,
        &CancellationToken::new(),
        None,
    )
    .await
}

/// Run a CLI command with full control over cancellation and progress.
///
/// - `cancel`: if cancelled while the child is running, the child is killed
///   (SIGTERM, then SIGKILL after a 2s grace period via `kill_on_drop`)
///   and [`Error::Cancelled`] is returned. Any output produced before
///   cancellation is forwarded to the sink but discarded from the return
///   value.
/// - `on_chunk`: optional sink invoked for every line of stdout/stderr as
///   it arrives. Useful for emitting MCP progress notifications.
///
/// Regardless of streaming, the full stdout/stderr is also accumulated
/// and returned in [`CliOutput`] for callers that need the final result.
pub async fn run_cli_streaming(
    binary: &CliBinary,
    args: &[&str],
    parse_json: bool,
    env: Option<&HashMap<String, String>>,
    cancel: &CancellationToken,
    on_chunk: Option<Arc<dyn ChunkSink>>,
) -> Result<CliOutput> {
    let mut cmd = Command::new(binary.command_name());
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Belt-and-braces: if this future is dropped (e.g. caller bails
        // before we finish handling cancel), tokio will SIGKILL the
        // child for us. Without this, a cancelled but un-awaited child
        // would survive as a zombie / runaway process.
        .kill_on_drop(true);

    if let Some(env_vars) = env {
        for (k, v) in env_vars {
            cmd.env(k, v);
        }
    }

    let mut child = cmd.spawn().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            binary.not_found_error()
        } else {
            Error::Io(e)
        }
    })?;

    let stdout = child.stdout.take().expect("stdout was piped");
    let stderr = child.stderr.take().expect("stderr was piped");

    let stdout_task = tokio::spawn(drain_stream(
        stdout,
        OutputStream::Stdout,
        on_chunk.clone(),
    ));
    let stderr_task = tokio::spawn(drain_stream(
        stderr,
        OutputStream::Stderr,
        on_chunk.clone(),
    ));

    // Wait for either: child exits, or cancellation fires.
    let status = tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            tracing::warn!(
                bin = binary.command_name(),
                "cancellation received; reaping process tree"
            );
            // SIGTERM the entire descendant tree (including any
            // in-container processes spawned via `devcontainer exec`
            // → `docker exec` → containerd-shim). Container PID
            // namespacing hides PIDs from each other but not from
            // the host, so /proc walking sees through it.
            if let Some(root_pid) = child.id() {
                crate::process_tree::reap(root_pid, std::time::Duration::from_secs(2)).await;
            }
            // The root child should now be dead; wait briefly to
            // collect its exit status and let the reader tasks observe
            // EOF on stdout/stderr. `kill_on_drop(true)` is our
            // backstop if any of this fails.
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                child.wait(),
            )
            .await;
            let _ = stdout_task.await;
            let _ = stderr_task.await;
            return Err(Error::Cancelled);
        }
        status = child.wait() => status?,
    };

    // Child exited; collect the accumulated output.
    let stdout_buf = stdout_task.await.unwrap_or_default();
    let stderr_buf = stderr_task.await.unwrap_or_default();
    let exit_code = status.code().unwrap_or(-1);

    let json = if parse_json {
        serde_json::from_str(&stdout_buf).ok()
    } else {
        None
    };

    Ok(CliOutput {
        exit_code,
        stdout: stdout_buf,
        stderr: stderr_buf,
        json,
    })
}

/// Read `reader` line-by-line, forwarding each line to `sink` (if set)
/// and accumulating into the returned buffer. EOF or error ends the loop.
async fn drain_stream<R: tokio::io::AsyncRead + Unpin + Send + 'static>(
    reader: R,
    stream: OutputStream,
    sink: Option<Arc<dyn ChunkSink>>,
) -> String {
    let mut lines = BufReader::new(reader).lines();
    let mut buf = String::new();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                if !buf.is_empty() {
                    buf.push('\n');
                }
                buf.push_str(&line);
                if let Some(sink) = sink.as_ref() {
                    sink.on_chunk(OutputChunk {
                        stream,
                        line,
                    })
                    .await;
                }
            }
            Ok(None) => break,
            Err(e) => {
                tracing::debug!(%e, ?stream, "error reading child output stream");
                break;
            }
        }
    }
    if !buf.is_empty() {
        buf.push('\n');
    }
    buf
}
