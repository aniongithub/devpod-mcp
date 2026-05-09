use devcontainer_mcp_core::{auth, codespaces};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CodespacesCreateParams {
    #[schemars(
        description = "GitHub auth handle from auth_status/auth_login (e.g. 'github-aniongithub')"
    )]
    auth: String,
    #[schemars(description = "Repository in owner/repo format")]
    repo: String,
    #[schemars(description = "Branch to create the codespace from")]
    branch: Option<String>,
    #[schemars(
        description = "Machine type — ask the user. Options: 'basicLinux32gb' (2 cores, 8 GB RAM), 'standardLinux32gb' (4 cores, 16 GB RAM), 'premiumLinux' (8 cores, 32 GB RAM), 'largePremiumLinux' (16 cores, 64 GB RAM)"
    )]
    machine: Option<String>,
    #[schemars(description = "Path to devcontainer.json within the repo")]
    devcontainer_path: Option<String>,
    #[schemars(description = "Display name for the codespace (max 48 chars)")]
    display_name: Option<String>,
    #[schemars(description = "Idle timeout before auto-stop, e.g. '10m', '1h'")]
    idle_timeout: Option<String>,
}

#[tool_router(router = codespaces_create_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "codespaces_create",
        description = "Create a new GitHub Codespace for a repository. Requires a GitHub auth handle (get one via auth_status or auth_login)."
    )]
    async fn codespaces_create(
        &self,
        Parameters(params): Parameters<CodespacesCreateParams>,
    ) -> String {
        let env = match auth::resolve_handle_env(&params.auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        match codespaces::create(
            &env,
            &params.repo,
            params.branch.as_deref(),
            params.machine.as_deref(),
            params.devcontainer_path.as_deref(),
            params.display_name.as_deref(),
            params.idle_timeout.as_deref(),
        )
        .await
        {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
