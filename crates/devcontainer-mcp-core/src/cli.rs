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

/// Drain a stream that may contain the [`crate::exec_shim`] sentinel.
///
/// Lines matching `__DCMCP_PGID=N__` are intercepted: `N` is stored
/// into `pgid_out` and the line is suppressed from both the sink and
/// the accumulated buffer. All other lines flow through normally.
async fn drain_stream_with_sentinel<R: tokio::io::AsyncRead + Unpin + Send + 'static>(
    reader: R,
    stream: OutputStream,
    sink: Option<Arc<dyn ChunkSink>>,
    pgid_out: Arc<std::sync::Mutex<Option<i32>>>,
) -> String {
    let mut lines = BufReader::new(reader).lines();
    let mut buf = String::new();
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                if let Some(found) = crate::exec_shim::try_parse_sentinel(&line) {
                    *pgid_out.lock().expect("pgid lock") = Some(found);
                    tracing::debug!(pgid = found, ?stream, "captured remote PGID");
                    continue;
                }
                if !buf.is_empty() {
                    buf.push('\n');
                }
                buf.push_str(&line);
                if let Some(sink) = sink.as_ref() {
                    sink.on_chunk(OutputChunk { stream, line }).await;
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

/// Trait for a backend-specific "send a kill into the remote target".
///
/// Used by [`run_with_shim`] to deliver SIGTERM/SIGKILL to a remote
/// process group when cancellation fires. The runner calls this with
/// the PGID captured from the shim's sentinel; the implementor knows
/// how to reach into the target (docker exec, devpod ssh, gh ssh, …).
///
/// Implementations should be *idempotent* and *forgiving*: a kill of
/// an already-exited process is fine, and the trait is invoked on a
/// best-effort basis.
#[async_trait::async_trait]
pub trait RemoteKiller: Send + Sync {
    /// Send `signal` (a POSIX signal name like "TERM" or "KILL") to
    /// the entire process group `pgid` on the remote target.
    async fn kill_pgid(&self, pgid: i32, signal: &str);
}

/// Run a CLI command whose remote target requires the [`crate::exec_shim`]
/// shim for cancellation.
///
/// Differences from [`run_cli_streaming`]:
///
/// - The stderr reader filters out the sentinel line emitted by the
///   shim and stores the captured remote PGID in shared state.
/// - On cancel, before reaping the host-side process tree, the helper
///   asks the supplied [`RemoteKiller`] to deliver SIGTERM then SIGKILL
///   to the captured PGID (with a 2-second grace between them). This
///   reaches workloads that have been reparented to the remote's init
///   (docker daemon's containerd-shim, sshd, …) and would otherwise
///   survive the death of our host-side wrapper.
///
/// If the sentinel never arrives (extremely fast cancellation, or a
/// target without `setsid`/base64), the helper falls back to host-tree
/// reaping only — the remote process *may* leak in that narrow window.
pub async fn run_with_shim(
    binary: &CliBinary,
    args: &[&str],
    env: Option<&HashMap<String, String>>,
    cancel: &CancellationToken,
    on_chunk: Option<Arc<dyn ChunkSink>>,
    killer: Arc<dyn RemoteKiller>,
) -> Result<CliOutput> {
    let mut cmd = Command::new(binary.command_name());
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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

    // Shared captured PGID. Filled in by the stderr reader as soon as
    // the shim's sentinel line arrives; consumed by the cancel branch.
    let pgid: Arc<std::sync::Mutex<Option<i32>>> = Arc::new(std::sync::Mutex::new(None));

    // stdout: no sentinel to scrub (the shim writes it to stderr).
    let stdout_task = tokio::spawn(drain_stream(
        stdout,
        OutputStream::Stdout,
        on_chunk.clone(),
    ));
    // stderr: filter the sentinel line, forward everything else.
    let stderr_task = tokio::spawn(drain_stream_with_sentinel(
        stderr,
        OutputStream::Stderr,
        on_chunk.clone(),
        pgid.clone(),
    ));

    let status = tokio::select! {
        biased;
        _ = cancel.cancelled() => {
            tracing::warn!(
                bin = binary.command_name(),
                "cancellation received; killing remote process group then host wrapper"
            );

            let captured = *pgid.lock().expect("pgid lock");
            match captured {
                Some(pgid_val) => {
                    // Remote kill first: if we kill the host wrapper
                    // first, the transport to the target closes and
                    // the remote work keeps running.
                    killer.kill_pgid(pgid_val, "TERM").await;
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    killer.kill_pgid(pgid_val, "KILL").await;
                }
                None => {
                    tracing::warn!(
                        bin = binary.command_name(),
                        "no remote PGID captured; relying on host reap (may leak)"
                    );
                }
            }

            // Host-side reap of the wrapper and any host descendants
            // we *can* see. For SSH-based backends this catches the
            // local SSH client process.
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
            return Err(Error::Cancelled);
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

#[cfg(test)]
mod shim_runner_tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;

    /// Records every `kill_pgid` invocation so tests can assert on
    /// the order and arguments.
    struct RecordingKiller {
        calls: Mutex<Vec<(i32, String)>>,
    }

    #[async_trait::async_trait]
    impl RemoteKiller for RecordingKiller {
        async fn kill_pgid(&self, pgid: i32, signal: &str) {
            self.calls.lock().unwrap().push((pgid, signal.to_string()));
        }
    }

    /// Sink that just counts received chunks.
    struct CountingSink {
        stdout_lines: AtomicU64,
        stderr_lines: AtomicU64,
    }
    #[async_trait::async_trait]
    impl ChunkSink for CountingSink {
        async fn on_chunk(&self, chunk: OutputChunk) {
            match chunk.stream {
                OutputStream::Stdout => {
                    self.stdout_lines.fetch_add(1, Ordering::Relaxed);
                }
                OutputStream::Stderr => {
                    self.stderr_lines.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    /// Create a `devpod` shim script in `dir` that ignores all args
    /// except the value after `--command` and runs that under `sh -c`.
    /// This lets us drive `run_with_shim` end-to-end without a real
    /// DevPod install — `binary.command_name()` is `devpod`, and our
    /// shim picks it up from `PATH`.
    fn install_devpod_shim(dir: &std::path::Path) -> std::path::PathBuf {
        let script = dir.join("devpod");
        // The fake devpod: parse out `--command <cmd>` and exec it
        // via `sh -c`. Everything else (workspace name, --user, etc.)
        // is discarded — we only care that the shim's body runs.
        let body = r#"#!/bin/sh
cmd=""
while [ $# -gt 0 ]; do
  case "$1" in
    --command)
      shift
      cmd="$1"
      shift
      ;;
    *)
      shift
      ;;
  esac
done
exec sh -c "$cmd"
"#;
        std::fs::write(&script, body).expect("write shim");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script, perms).unwrap();
        }
        script
    }

    /// End-to-end: shim wraps a sleep, runner captures PGID from
    /// stderr, cancellation triggers RemoteKiller in the right order
    /// (TERM then KILL with a 2s gap), and the call returns
    /// Error::Cancelled.
    ///
    /// This exercises the full `run_with_shim` flow without docker
    /// or any real backend — the `RemoteKiller` is a recording fake
    /// and the "remote" is just a local shell invoked by a stub
    /// `devpod` shim on PATH.
    #[tokio::test]
    async fn run_with_shim_invokes_killer_in_order_on_cancel() {
        let tmp = tempfile::tempdir().expect("tmpdir");
        install_devpod_shim(tmp.path());
        let new_path = format!(
            "{}:{}",
            tmp.path().display(),
            std::env::var("PATH").unwrap_or_default()
        );
        // Safe because tests in this module are single-threaded by
        // tokio::test's default runtime; if we move to multi_thread
        // we'd need a process-wide PATH lock.
        // SAFETY: see comment above.
        unsafe {
            std::env::set_var("PATH", &new_path);
        }

        let killer = Arc::new(RecordingKiller {
            calls: Mutex::new(Vec::new()),
        });
        let sink = Arc::new(CountingSink {
            stdout_lines: AtomicU64::new(0),
            stderr_lines: AtomicU64::new(0),
        });
        let cancel = CancellationToken::new();

        // The "user command" — backgrounded sleeps + wait. The shim
        // wraps this so the runner sees the sentinel on stderr.
        let user_cmd = "sleep 30 & sleep 30 & wait";
        let wrapped = crate::exec_shim::wrap_self_contained(user_cmd);

        // Spawn the call and cancel it shortly after.
        let killer_for_spawn: Arc<dyn RemoteKiller> = killer.clone();
        let sink_for_spawn: Arc<dyn ChunkSink> = sink.clone();
        let cancel_for_spawn = cancel.clone();
        let handle = tokio::spawn(async move {
            let args = vec!["ssh", "fake-ws", "--command", wrapped.as_str()];
            run_with_shim(
                &CliBinary::DevPod,
                &args,
                None,
                &cancel_for_spawn,
                Some(sink_for_spawn),
                killer_for_spawn,
            )
            .await
        });

        // Give the shim time to emit the sentinel.
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        cancel.cancel();

        let result = handle.await.expect("join");
        match result {
            Err(Error::Cancelled) => { /* expected */ }
            other => panic!("expected Error::Cancelled, got {:?}", other),
        }

        // Killer should have been called once with TERM, then once
        // with KILL, both targeting the same PGID (>0). The 2s gap
        // is checked separately below.
        let calls = killer.calls.lock().unwrap().clone();
        assert_eq!(calls.len(), 2, "expected exactly 2 kill calls, got {calls:?}");
        let (pgid1, sig1) = &calls[0];
        let (pgid2, sig2) = &calls[1];
        assert_eq!(sig1, "TERM");
        assert_eq!(sig2, "KILL");
        assert!(*pgid1 > 0, "captured PGID should be positive, got {pgid1}");
        assert_eq!(pgid1, pgid2, "both signals should target the same PGID");

        // Sentinel must NOT have leaked into the sink's stderr count
        // — if it had, sink.stderr_lines would include at least the
        // sentinel line for every chunk. Concretely, the user_cmd
        // produces no stderr at all, so stderr_lines should be 0.
        assert_eq!(
            sink.stderr_lines.load(Ordering::Relaxed),
            0,
            "sentinel should be scrubbed from stderr forwarded to sink"
        );
    }
}
