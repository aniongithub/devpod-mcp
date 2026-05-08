mod create;
mod delete;
mod files;
mod list;
mod ports;
mod ssh;
mod stop;
mod view;

use rmcp::handler::server::router::tool::ToolRouter;

use super::DevContainerMcp;

impl DevContainerMcp {
    pub(super) fn codespaces_router() -> ToolRouter<Self> {
        Self::codespaces_create_router()
            + Self::codespaces_list_router()
            + Self::codespaces_ssh_router()
            + Self::codespaces_stop_router()
            + Self::codespaces_delete_router()
            + Self::codespaces_view_router()
            + Self::codespaces_ports_router()
            + Self::codespaces_files_router()
    }
}
