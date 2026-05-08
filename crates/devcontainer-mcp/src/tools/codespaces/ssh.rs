use devcontainer_mcp_core::{auth, codespaces};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CodespacesSshParams {
    #[schemars(description = "GitHub auth handle (e.g. 'github-aniongithub')")]
    auth: String,
    #[schemars(description = "Codespace name (from codespaces_list or codespaces_create)")]
    codespace: String,
    #[schemars(description = "Command to execute inside the codespace")]
    command: String,
}

#[tool_router(router = codespaces_ssh_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "codespaces_ssh",
        description = "Execute a command inside a GitHub Codespace via SSH. Requires a GitHub auth handle."
    )]
    async fn codespaces_ssh(&self, Parameters(params): Parameters<CodespacesSshParams>) -> String {
        let env = match auth::resolve_handle_env(&params.auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        match codespaces::ssh_exec(&env, &params.codespace, &params.command).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
