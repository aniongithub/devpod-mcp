use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::wsl;
use rmcp::{tool, tool_router};

#[tool_router(router = wsl_list_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "wsl_list",
        description = "List WSL distributions with their state and version."
    )]
    async fn wsl_list(&self) -> String {
        match wsl::list().await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
