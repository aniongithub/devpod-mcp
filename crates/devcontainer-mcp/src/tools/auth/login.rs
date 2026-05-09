use devcontainer_mcp_core::auth;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct AuthLoginParams {
    #[schemars(description = "Auth provider name (e.g. 'github')")]
    provider: String,
    #[schemars(description = "Additional OAuth scopes to request (e.g. 'codespace' for GitHub)")]
    scopes: Option<String>,
}

#[tool_router(router = auth_login_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "auth_login",
        description = "Initiate authentication for a provider. Opens browser, copies device code to clipboard, and waits for approval. Returns an auth handle on success."
    )]
    async fn auth_login(&self, Parameters(params): Parameters<AuthLoginParams>) -> String {
        match auth::get_provider(&params.provider) {
            Some(p) => match p.login(params.scopes.as_deref()).await {
                Ok(result) => {
                    serde_json::to_string(&result).unwrap_or_else(|e| format!("Error: {e}"))
                }
                Err(e) => format!("Error: {e}"),
            },
            None => format!("Unknown auth provider: {}", params.provider),
        }
    }
}
