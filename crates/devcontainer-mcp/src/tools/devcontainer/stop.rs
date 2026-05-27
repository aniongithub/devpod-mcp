use devcontainer_mcp_core::devcontainer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerStopParams {
    #[schemars(description = "Path to the workspace folder (used to find the container by label)")]
    workspace_folder: String,
    #[schemars(
        description = "Path to a specific devcontainer.json (use to disambiguate multi-container workspaces)"
    )]
    config: Option<String>,
}

#[tool_router(router = devcontainer_stop_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devcontainer_stop",
        description = "Stop a running local dev container (via Docker). The devcontainer CLI has no stop command, so this uses the Docker API directly."
    )]
    async fn devcontainer_stop(
        &self,
        Parameters(params): Parameters<DevcontainerStopParams>,
    ) -> String {
        match devcontainer::stop(&params.workspace_folder, params.config.as_deref()).await {
            Ok(msg) => msg,
            Err(e) => format!("Error: {e}"),
        }
    }
}
