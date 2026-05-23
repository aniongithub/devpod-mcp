use std::sync::Arc;

use devcontainer_mcp_core::cli::{ChunkSink, OutputChunk, OutputStream};
use devcontainer_mcp_core::devpod;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Meta, ProgressNotificationParam, ProgressToken};
use rmcp::service::Peer;
use rmcp::{tool, tool_router, RoleServer};
use tokio_util::sync::CancellationToken;

use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodSshParams {
    #[schemars(description = "Workspace name or ID")]
    workspace: String,
    #[schemars(description = "Command to execute inside the workspace")]
    command: String,
    #[serde(default)]
    #[schemars(description = "User to run the command as")]
    user: Option<String>,
    #[serde(default)]
    #[schemars(description = "Working directory inside the workspace")]
    workdir: Option<String>,
}

/// Forwards every line of child output to the MCP peer as a
/// `notifications/progress` message, keeping the transport warm during
/// long-running execs.
struct ProgressChunkSink {
    peer: Peer<RoleServer>,
    progress_token: ProgressToken,
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
            tracing::debug!(%e, "failed to send progress notification");
        }
    }
}

#[tool_router(router = devpod_ssh_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devpod_ssh",
        description = "Execute a command inside a DevPod workspace via SSH. Returns stdout, stderr, and exit code."
    )]
    async fn devpod_ssh(
        &self,
        Parameters(params): Parameters<DevpodSshParams>,
        ct: CancellationToken,
        peer: Peer<RoleServer>,
        meta: Meta,
    ) -> String {
        let sink: Option<Arc<dyn ChunkSink>> = meta.get_progress_token().map(|token| {
            Arc::new(ProgressChunkSink {
                peer: peer.clone(),
                progress_token: token,
                counter: std::sync::atomic::AtomicU64::new(0),
            }) as Arc<dyn ChunkSink>
        });

        match devpod::ssh_exec_streaming(
            &params.workspace,
            &params.command,
            params.user.as_deref(),
            params.workdir.as_deref(),
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
