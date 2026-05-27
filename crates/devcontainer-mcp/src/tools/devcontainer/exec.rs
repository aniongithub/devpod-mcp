use devcontainer_mcp_core::devcontainer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::Meta;
use rmcp::service::Peer;
use rmcp::{tool, tool_router, RoleServer};
use tokio_util::sync::CancellationToken;

use crate::tools::common::{format_output, progress_sink_from_meta};
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerExecParams {
    #[schemars(description = "Path to the workspace folder")]
    workspace_folder: String,
    #[schemars(
        description = "Path to a specific devcontainer.json (use to disambiguate multi-container workspaces)"
    )]
    config: Option<String>,
    #[schemars(description = "Command to execute inside the container")]
    command: String,
    #[schemars(description = "Arguments for the command as a space-separated string")]
    args: Option<String>,
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

        let sink = progress_sink_from_meta(&meta, &peer);

        match devcontainer::exec_streaming(
            &params.workspace_folder,
            params.config.as_deref(),
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
