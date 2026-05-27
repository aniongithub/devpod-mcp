mod build;
mod config;
mod exec;
mod files;
mod list_configs;
mod remove;
mod status;
mod stop;
mod up;

use rmcp::handler::server::router::tool::ToolRouter;

use super::DevContainerMcp;

impl DevContainerMcp {
    pub(super) fn devcontainer_router() -> ToolRouter<Self> {
        Self::devcontainer_up_router()
            + Self::devcontainer_exec_router()
            + Self::devcontainer_build_router()
            + Self::devcontainer_config_router()
            + Self::devcontainer_stop_router()
            + Self::devcontainer_remove_router()
            + Self::devcontainer_status_router()
            + Self::devcontainer_files_router()
            + Self::devcontainer_list_configs_router()
    }
}
