use devcontainer_mcp_core::{auth, codespaces};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::Meta;
use rmcp::service::Peer;
use rmcp::{tool, tool_router, RoleServer};
use tokio_util::sync::CancellationToken;

use crate::tools::common::{format_output, progress_sink_from_meta};
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

        let sink = progress_sink_from_meta(&meta, &peer);

        match codespaces::ssh_exec_streaming(&env, &params.codespace, &params.command, &ct, sink)
            .await
        {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
