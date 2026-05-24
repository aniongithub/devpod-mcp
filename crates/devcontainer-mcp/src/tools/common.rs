use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use devcontainer_mcp_core::cli::{ChunkSink, CliOutput, OutputChunk, OutputStream};
use rmcp::model::{Meta, ProgressNotificationParam, ProgressToken};
use rmcp::service::Peer;
use rmcp::RoleServer;

/// Format a CliOutput as a JSON string for MCP responses.
pub fn format_output(output: &CliOutput) -> String {
    serde_json::json!({
        "exit_code": output.exit_code,
        "stdout": output.stdout,
        "stderr": output.stderr,
        "json": output.json,
    })
    .to_string()
}

/// Forwards every line of child output to the MCP peer as a
/// `notifications/progress` message. Used by all long-running tools
/// (`*_exec`, `*_ssh`, `*_up`, `*_build`, `codespaces_create`) so
/// clients can render progress and so idle-based client timeouts
/// don't trip on multi-minute operations.
///
/// The MCP spec requires a strictly-increasing `progress` value on
/// every notification with the same token, hence the per-sink
/// atomic counter.
struct ProgressChunkSink {
    peer: Peer<RoleServer>,
    progress_token: ProgressToken,
    counter: AtomicU64,
}

#[async_trait::async_trait]
impl ChunkSink for ProgressChunkSink {
    async fn on_chunk(&self, chunk: OutputChunk) {
        let n = self.counter.fetch_add(1, Ordering::Relaxed) + 1;
        let prefix = match chunk.stream {
            OutputStream::Stdout => "stdout",
            OutputStream::Stderr => "stderr",
        };
        let param = ProgressNotificationParam::new(self.progress_token.clone(), n as f64)
            .with_message(format!("{prefix}: {}", chunk.line));
        if let Err(e) = self.peer.notify_progress(param).await {
            // Sink errors are non-fatal: the peer may have
            // disconnected or simply not subscribed to progress.
            // We still want the child to keep running and produce
            // a final response (which may or may not get delivered).
            tracing::debug!(%e, "failed to send progress notification");
        }
    }
}

/// Build a [`ChunkSink`] from a request's `_meta`.
///
/// Returns `Some(Arc<dyn ChunkSink>)` if the client supplied a
/// `progressToken`, `None` otherwise. Tool handlers should pass the
/// returned sink straight into `*_streaming` core functions.
pub fn progress_sink_from_meta(
    meta: &Meta,
    peer: &Peer<RoleServer>,
) -> Option<Arc<dyn ChunkSink>> {
    meta.get_progress_token().map(|token| {
        Arc::new(ProgressChunkSink {
            peer: peer.clone(),
            progress_token: token,
            counter: AtomicU64::new(0),
        }) as Arc<dyn ChunkSink>
    })
}
