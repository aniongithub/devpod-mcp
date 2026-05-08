use devcontainer_mcp_core::devcontainer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerRemoveParams {
    #[schemars(description = "Path to the workspace folder (used to find the container by label)")]
    workspace_folder: String,
    #[schemars(description = "Force removal even if the container is running")]
    force: Option<bool>,
}

#[tool_router(router = devcontainer_remove_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devcontainer_remove",
        description = "Remove a local dev container and its resources (via Docker). Stops the container first if running."
    )]
    async fn devcontainer_remove(
        &self,
        Parameters(params): Parameters<DevcontainerRemoveParams>,
    ) -> String {
        match devcontainer::remove(&params.workspace_folder, params.force.unwrap_or(false)).await {
            Ok(msg) => msg,
            Err(e) => format!("Error: {e}"),
        }
    }
}
