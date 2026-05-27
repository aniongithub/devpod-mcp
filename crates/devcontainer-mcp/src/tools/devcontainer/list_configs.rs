use devcontainer_mcp_core::devcontainer_config;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerListConfigsParams {
    #[schemars(description = "Path to the workspace folder to scan for devcontainer.json files")]
    workspace_folder: String,
}

#[tool_router(router = devcontainer_list_configs_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devcontainer_list_configs",
        description = "Enumerate devcontainer.json files in a workspace. Returns each discovered config (root `.devcontainer.json`, `.devcontainer/devcontainer.json`, and `.devcontainer/*/devcontainer.json`) with its parsed name, image, service, dockerComposeFile, workspaceFolder, and kind (compose | image | dockerfile | unknown). Use the returned `path` directly as the `config` parameter to other devcontainer tools to target a specific container in a multi-container workspace."
    )]
    async fn devcontainer_list_configs(
        &self,
        Parameters(params): Parameters<DevcontainerListConfigsParams>,
    ) -> String {
        match devcontainer_config::list_configs(&params.workspace_folder) {
            Ok(entries) => {
                serde_json::to_string(&entries).unwrap_or_else(|e| format!("Error: {e}"))
            }
            Err(e) => format!("Error: {e}"),
        }
    }
}
