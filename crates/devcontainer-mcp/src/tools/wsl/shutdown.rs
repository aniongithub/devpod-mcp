use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::wsl;
use rmcp::{tool, tool_router};

#[tool_router(router = wsl_shutdown_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "wsl_shutdown",
        description = "Shut down all running WSL distributions."
    )]
    async fn wsl_shutdown(&self) -> String {
        match wsl::shutdown().await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
