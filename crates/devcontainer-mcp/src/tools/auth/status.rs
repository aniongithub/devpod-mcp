use devcontainer_mcp_core::auth;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct AuthStatusParams {
    #[schemars(description = "Auth provider name (e.g. 'github', 'aws', 'azure', 'gcloud')")]
    provider: String,
}

#[tool_router(router = auth_status_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "auth_status",
        description = "Check authentication status for a provider. Returns available auth handles and account info. Providers: 'github', 'aws', 'azure', 'gcloud', 'kubernetes'."
    )]
    async fn auth_status(&self, Parameters(params): Parameters<AuthStatusParams>) -> String {
        match auth::get_provider(&params.provider) {
            Some(p) => match p.status().await {
                Ok(status) => {
                    serde_json::to_string(&status).unwrap_or_else(|e| format!("Error: {e}"))
                }
                Err(e) => format!("Error: {e}"),
            },
            None => format!("Unknown auth provider: {}", params.provider),
        }
    }
}
