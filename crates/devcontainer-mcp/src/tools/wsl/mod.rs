mod exec;
mod files;
mod list;
mod set_default;
mod shutdown;
mod stop;

use rmcp::handler::server::router::tool::ToolRouter;

use super::DevContainerMcp;

impl DevContainerMcp {
    pub(super) fn wsl_router() -> ToolRouter<Self> {
        Self::wsl_list_router()
            + Self::wsl_exec_router()
            + Self::wsl_stop_router()
            + Self::wsl_shutdown_router()
            + Self::wsl_set_default_router()
            + Self::wsl_files_router()
    }
}
