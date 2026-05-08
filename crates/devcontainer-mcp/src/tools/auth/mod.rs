mod login;
mod logout;
mod select;
mod status;

use rmcp::handler::server::router::tool::ToolRouter;

use super::DevContainerMcp;

impl DevContainerMcp {
    pub(super) fn auth_router() -> ToolRouter<Self> {
        Self::auth_status_router()
            + Self::auth_login_router()
            + Self::auth_select_router()
            + Self::auth_logout_router()
    }
}
