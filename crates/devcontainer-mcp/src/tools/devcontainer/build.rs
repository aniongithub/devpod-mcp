use devcontainer_mcp_core::devcontainer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerBuildParams {
    #[schemars(description = "Path to the workspace folder")]
    workspace_folder: String,
    #[schemars(
        description = "Additional flags as space-separated args, e.g. '--no-cache --image-name my-image'"
    )]
    extra_args: Option<String>,
}

#[tool_router(router = devcontainer_build_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devcontainer_build",
        description = "Build a dev container image without starting it."
    )]
    async fn devcontainer_build(
        &self,
        Parameters(params): Parameters<DevcontainerBuildParams>,
    ) -> String {
        let extra: Vec<&str> = params
            .extra_args
            .as_deref()
            .map(|a| a.split_whitespace().collect())
            .unwrap_or_default();
        match devcontainer::build(&params.workspace_folder, &extra).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
