use devcontainer_mcp_core::devcontainer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerReadConfigParams {
    #[schemars(description = "Path to the workspace folder")]
    workspace_folder: String,
    #[schemars(description = "Path to a specific devcontainer.json")]
    config: Option<String>,
}

#[tool_router(router = devcontainer_config_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devcontainer_read_config",
        description = "Read and return the merged devcontainer configuration as JSON."
    )]
    async fn devcontainer_read_config(
        &self,
        Parameters(params): Parameters<DevcontainerReadConfigParams>,
    ) -> String {
        match devcontainer::read_configuration(&params.workspace_folder, params.config.as_deref())
            .await
        {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
