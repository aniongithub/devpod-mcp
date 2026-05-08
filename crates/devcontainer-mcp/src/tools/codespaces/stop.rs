use devcontainer_mcp_core::{auth, codespaces};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CodespacesStopParams {
    #[schemars(description = "GitHub auth handle (e.g. 'github-aniongithub')")]
    auth: String,
    #[schemars(description = "Codespace name")]
    codespace: String,
}

#[tool_router(router = codespaces_stop_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "codespaces_stop",
        description = "Stop a running GitHub Codespace. Requires a GitHub auth handle."
    )]
    async fn codespaces_stop(
        &self,
        Parameters(params): Parameters<CodespacesStopParams>,
    ) -> String {
        let env = match auth::resolve_handle_env(&params.auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        match codespaces::stop(&env, &params.codespace).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
