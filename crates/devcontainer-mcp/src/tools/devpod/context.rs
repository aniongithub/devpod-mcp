use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::devpod;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodContextUseParams {
    #[schemars(description = "Context name to switch to")]
    context: String,
}

#[tool_router(router = devpod_context_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devpod_context_list",
        description = "List all DevPod contexts."
    )]
    async fn devpod_context_list(&self) -> String {
        match devpod::context_list().await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devpod_context_use",
        description = "Switch to a different DevPod context."
    )]
    async fn devpod_context_use(
        &self,
        Parameters(params): Parameters<DevpodContextUseParams>,
    ) -> String {
        match devpod::context_use(&params.context).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
