use devcontainer_mcp_core::{auth, codespaces};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CodespacesListParams {
    #[schemars(description = "GitHub auth handle (e.g. 'github-aniongithub')")]
    auth: String,
    #[schemars(description = "Filter by repository (owner/repo format)")]
    repo: Option<String>,
}

#[tool_router(router = codespaces_list_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "codespaces_list",
        description = "List your GitHub Codespaces. Requires a GitHub auth handle."
    )]
    async fn codespaces_list(
        &self,
        Parameters(params): Parameters<CodespacesListParams>,
    ) -> String {
        let env = match auth::resolve_handle_env(&params.auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        match codespaces::list(&env, params.repo.as_deref()).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
