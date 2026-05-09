use devcontainer_mcp_core::auth;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct AuthSelectParams {
    #[schemars(description = "Auth handle to switch to (e.g. 'github-aniongithub', 'aws-prod')")]
    id: String,
}

#[tool_router(router = auth_select_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "auth_select",
        description = "Switch the active account for a provider. Returns account info if successful, null if the handle is invalid."
    )]
    async fn auth_select(&self, Parameters(params): Parameters<AuthSelectParams>) -> String {
        let provider_name = auth::provider_from_handle(&params.id).unwrap_or("unknown");
        match auth::get_provider(provider_name) {
            Some(p) => match p.select(&params.id).await {
                Ok(Some(account)) => {
                    serde_json::to_string(&account).unwrap_or_else(|e| format!("Error: {e}"))
                }
                Ok(None) => format!("Failed to switch to: {}", params.id),
                Err(e) => format!("Error: {e}"),
            },
            None => format!("Unknown auth provider in handle: {}", params.id),
        }
    }
}
