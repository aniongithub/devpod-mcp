use std::sync::Arc;

use devcontainer_mcp_core::cli::{ChunkSink, OutputChunk, OutputStream};
use devcontainer_mcp_core::devcontainer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Meta, ProgressNotificationParam, ProgressToken};
use rmcp::service::{Peer, RequestContext};
use rmcp::{tool, tool_router, RoleServer};
use tokio_util::sync::CancellationToken;

use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerExecParams {
    #[schemars(description = "Path to the workspace folder")]
    workspace_folder: String,
    #[schemars(description = "Command to execute inside the container")]
    command: String,
    #[schemars(description = "Arguments for the command as a space-separated string")]
    args: Option<String>,
}

/// Forwards every line of child output to the MCP peer as a
/// `notifications/progress` message. Used to keep the wire warm during
/// long-running execs (e.g. `go test -race`) so client-side watchdogs
/// don't tear down the transport.
struct ProgressChunkSink {
    peer: Peer<RoleServer>,
    progress_token: ProgressToken,
    /// Monotonic counter so each notification has a strictly-increasing
    /// `progress` value, as required by the MCP spec.
    counter: std::sync::atomic::AtomicU64,
}

#[async_trait::async_trait]
impl ChunkSink for ProgressChunkSink {
    async fn on_chunk(&self, chunk: OutputChunk) {
        let n = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        let prefix = match chunk.stream {
            OutputStream::Stdout => "stdout",
            OutputStream::Stderr => "stderr",
        };
        let param = ProgressNotificationParam::new(self.progress_token.clone(), n as f64)
            .with_message(format!("{prefix}: {}", chunk.line));
        if let Err(e) = self.peer.notify_progress(param).await {
            // Sink errors are non-fatal: the peer may have disconnected
            // or simply not subscribed to progress. We still want the
            // child to keep running and produce a final response (which
            // may or may not get delivered).
            tracing::debug!(%e, "failed to send progress notification");
        }
    }
}

#[tool_router(router = devcontainer_exec_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devcontainer_exec",
        description = "Execute a command inside a running local dev container."
    )]
    async fn devcontainer_exec(
        &self,
        Parameters(params): Parameters<DevcontainerExecParams>,
        ct: CancellationToken,
        peer: Peer<RoleServer>,
        meta: Meta,
    ) -> String {
        let full_cmd = match &params.args {
            Some(a) => format!("{} {}", params.command, a),
            None => params.command,
        };

        // If the client supplied a progressToken in _meta, stream
        // stdout/stderr to it as progress notifications. Otherwise just
        // run with cancellation but no streaming.
        let sink: Option<Arc<dyn ChunkSink>> = meta.get_progress_token().map(|token| {
            Arc::new(ProgressChunkSink {
                peer: peer.clone(),
                progress_token: token,
                counter: std::sync::atomic::AtomicU64::new(0),
            }) as Arc<dyn ChunkSink>
        });

        match devcontainer::exec_streaming(
            &params.workspace_folder,
            "sh",
            &["-c", &full_cmd],
            &ct,
            sink,
        )
        .await
        {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}

// `RequestContext` is not strictly required here — `CancellationToken`,
// `Peer<RoleServer>`, and `Meta` are extracted directly by the tool_router
// macro via their respective `FromContextPart` impls — but importing it
// once keeps it discoverable for future tool authors.
#[allow(dead_code)]
fn _request_context_marker(_: RequestContext<RoleServer>) {}
