use std::sync::Arc;

use devcontainer_mcp_core::cli::{ChunkSink, OutputChunk, OutputStream};
use devcontainer_mcp_core::{auth, codespaces};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{Meta, ProgressNotificationParam, ProgressToken};
use rmcp::service::Peer;
use rmcp::{tool, tool_router, RoleServer};
use tokio_util::sync::CancellationToken;

use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CodespacesSshParams {
    #[schemars(description = "GitHub auth handle (e.g. 'github-aniongithub')")]
    auth: String,
    #[schemars(description = "Codespace name (from codespaces_list or codespaces_create)")]
    codespace: String,
    #[schemars(description = "Command to execute inside the codespace")]
    command: String,
}

/// See the devcontainer/exec.rs version for full commentary; this is a
/// near-identical copy specialized to the Codespaces backend.
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

#[tool_router(router = codespaces_ssh_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "codespaces_ssh",
        description = "Execute a command inside a GitHub Codespace via SSH. Requires a GitHub auth handle."
    )]
    async fn codespaces_ssh(
        &self,
        Parameters(params): Parameters<CodespacesSshParams>,
        ct: CancellationToken,
        peer: Peer<RoleServer>,
        meta: Meta,
    ) -> String {
        let env = match auth::resolve_handle_env(&params.auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };

        let sink: Option<Arc<dyn ChunkSink>> = meta.get_progress_token().map(|token| {
            Arc::new(ProgressChunkSink {
                peer: peer.clone(),
                progress_token: token,
                counter: std::sync::atomic::AtomicU64::new(0),
            }) as Arc<dyn ChunkSink>
        });

        match codespaces::ssh_exec_streaming(
            &env,
            &params.codespace,
            &params.command,
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
