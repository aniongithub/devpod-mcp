mod build;
mod container;
mod context;
mod delete;
mod files;
mod list;
mod logs;
mod provider;
mod ssh;
mod status;
mod stop;
mod up;

use rmcp::handler::server::router::tool::ToolRouter;

use super::DevContainerMcp;

impl DevContainerMcp {
    pub(super) fn devpod_router() -> ToolRouter<Self> {
        Self::devpod_up_router()
            + Self::devpod_stop_router()
            + Self::devpod_delete_router()
            + Self::devpod_build_router()
            + Self::devpod_status_router()
            + Self::devpod_list_router()
            + Self::devpod_ssh_router()
            + Self::devpod_logs_router()
            + Self::devpod_provider_router()
            + Self::devpod_context_router()
            + Self::devpod_container_router()
            + Self::devpod_files_router()
    }
}
