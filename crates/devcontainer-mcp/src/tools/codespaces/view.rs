use devcontainer_mcp_core::{auth, codespaces};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CodespacesViewParams {
    #[schemars(description = "GitHub auth handle (e.g. 'github-aniongithub')")]
    auth: String,
    #[schemars(description = "Codespace name")]
    codespace: String,
}

#[tool_router(router = codespaces_view_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "codespaces_view",
        description = "View detailed information about a GitHub Codespace. Requires a GitHub auth handle."
    )]
    async fn codespaces_view(
        &self,
        Parameters(params): Parameters<CodespacesViewParams>,
    ) -> String {
        let env = match auth::resolve_handle_env(&params.auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        match codespaces::view(&env, &params.codespace).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
