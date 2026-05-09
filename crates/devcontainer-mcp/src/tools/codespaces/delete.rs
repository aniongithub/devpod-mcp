use devcontainer_mcp_core::{auth, codespaces};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CodespacesDeleteParams {
    #[schemars(description = "GitHub auth handle (e.g. 'github-aniongithub')")]
    auth: String,
    #[schemars(description = "Codespace name")]
    codespace: String,
    #[schemars(description = "Force delete even with unsaved changes")]
    force: Option<bool>,
}

#[tool_router(router = codespaces_delete_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "codespaces_delete",
        description = "Delete a GitHub Codespace. Requires a GitHub auth handle."
    )]
    async fn codespaces_delete(
        &self,
        Parameters(params): Parameters<CodespacesDeleteParams>,
    ) -> String {
        let env = match auth::resolve_handle_env(&params.auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        match codespaces::delete(&env, &params.codespace, params.force.unwrap_or(false)).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
