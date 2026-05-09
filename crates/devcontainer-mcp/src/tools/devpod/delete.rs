use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::devpod;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodDeleteParams {
    #[schemars(description = "Workspace name or ID")]
    workspace: String,
    #[serde(default)]
    #[schemars(description = "Force delete even if workspace is not found remotely")]
    force: Option<bool>,
}

#[tool_router(router = devpod_delete_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devpod_delete",
        description = "Delete a DevPod workspace. Stops and removes all associated resources."
    )]
    async fn devpod_delete(&self, Parameters(params): Parameters<DevpodDeleteParams>) -> String {
        match devpod::delete(&params.workspace, params.force.unwrap_or(false)).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
