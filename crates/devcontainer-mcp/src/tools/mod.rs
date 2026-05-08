mod auth;
mod codespaces;
pub mod common;
mod devcontainer;
mod devpod;
#[cfg(target_os = "windows")]
mod wsl;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{tool_handler, ServerHandler};

#[derive(Debug, Clone)]
pub struct DevContainerMcp;

impl DevContainerMcp {
    pub fn new() -> Self {
        Self
    }

    fn combined_router() -> ToolRouter<Self> {
        let r = Self::devpod_router()
            + Self::devcontainer_router()
            + Self::codespaces_router()
            + Self::auth_router();
        #[cfg(target_os = "windows")]
        let r = r + Self::wsl_router();
        r
    }
}

#[tool_handler(router = Self::combined_router())]
impl ServerHandler for DevContainerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(rmcp::model::Implementation::new(
                "devcontainer-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "DevContainer MCP — a unified MCP server for managing dev containers across \
                 multiple backends. Supports DevPod (devpod_* tools), the devcontainer CLI \
                 (devcontainer_* tools), and GitHub Codespaces (codespaces_* tools). \
                 Use the appropriate tool prefix based on the backend you want to use.",
            )
    }
}
