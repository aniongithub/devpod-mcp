use devcontainer_mcp_core::devcontainer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerStatusParams {
    #[schemars(description = "Path to the workspace folder")]
    workspace_folder: String,
}

#[tool_router(router = devcontainer_status_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devcontainer_status",
        description = "Get the status of a local dev container. Returns container info (state, image, labels) or null if not found."
    )]
    async fn devcontainer_status(
        &self,
        Parameters(params): Parameters<DevcontainerStatusParams>,
    ) -> String {
        match devcontainer::status(&params.workspace_folder).await {
            Ok(Some(info)) => {
                serde_json::to_string(&info).unwrap_or_else(|e| format!("Error: {e}"))
            }
            Ok(None) => r#"{"state":"NotFound"}"#.to_string(),
            Err(e) => format!("Error: {e}"),
        }
    }
}
