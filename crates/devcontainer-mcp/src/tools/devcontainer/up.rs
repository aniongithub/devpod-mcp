use devcontainer_mcp_core::devcontainer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerUpParams {
    #[schemars(
        description = "Path to the workspace folder containing .devcontainer/devcontainer.json"
    )]
    workspace_folder: String,
    #[schemars(description = "Path to a specific devcontainer.json (overrides auto-detection)")]
    config: Option<String>,
    #[schemars(
        description = "Additional flags as space-separated args, e.g. '--remove-existing-container --build-no-cache'"
    )]
    extra_args: Option<String>,
}

#[tool_router(router = devcontainer_up_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devcontainer_up",
        description = "Create and start a local dev container using the devcontainer CLI. Requires a workspace folder with a devcontainer.json."
    )]
    async fn devcontainer_up(
        &self,
        Parameters(params): Parameters<DevcontainerUpParams>,
    ) -> String {
        let extra: Vec<&str> = params
            .extra_args
            .as_deref()
            .map(|a| a.split_whitespace().collect())
            .unwrap_or_default();
        match devcontainer::up(&params.workspace_folder, params.config.as_deref(), &extra).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
