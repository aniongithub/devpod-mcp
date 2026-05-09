use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::devpod;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodStopParams {
    #[schemars(description = "Workspace name or ID")]
    workspace: String,
}

#[tool_router(router = devpod_stop_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(name = "devpod_stop", description = "Stop a running DevPod workspace.")]
    async fn devpod_stop(&self, Parameters(params): Parameters<DevpodStopParams>) -> String {
        match devpod::stop(&params.workspace).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
