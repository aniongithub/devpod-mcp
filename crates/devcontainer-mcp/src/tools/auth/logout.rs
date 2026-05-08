use devcontainer_mcp_core::auth;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct AuthLogoutParams {
    #[schemars(
        description = "Auth handle to logout (e.g. 'github-aniongithub', 'azure-<sub-id>')"
    )]
    id: String,
}

#[tool_router(router = auth_logout_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "auth_logout",
        description = "Logout / revoke an authenticated account. Removes credentials from the provider's keyring."
    )]
    async fn auth_logout(&self, Parameters(params): Parameters<AuthLogoutParams>) -> String {
        let provider_name = auth::provider_from_handle(&params.id).unwrap_or("unknown");
        match auth::get_provider(provider_name) {
            Some(p) => match p.logout(&params.id).await {
                Ok(msg) => msg,
                Err(e) => format!("Error: {e}"),
            },
            None => format!("Unknown auth provider in handle: {}", params.id),
        }
    }
}
