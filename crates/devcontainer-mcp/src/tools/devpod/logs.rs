use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::devpod;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodLogsParams {
    #[schemars(description = "Workspace name or ID")]
    workspace: String,
}

#[tool_router(router = devpod_logs_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devpod_logs",
        description = "Get logs from a DevPod workspace."
    )]
    async fn devpod_logs(&self, Parameters(params): Parameters<DevpodLogsParams>) -> String {
        match devpod::logs(&params.workspace).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
