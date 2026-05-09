use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::devpod;
use rmcp::{tool, tool_router};

#[tool_router(router = devpod_list_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devpod_list",
        description = "List all DevPod workspaces. Returns JSON array with workspace IDs, sources, providers, and status."
    )]
    async fn devpod_list(&self) -> String {
        match devpod::list().await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
