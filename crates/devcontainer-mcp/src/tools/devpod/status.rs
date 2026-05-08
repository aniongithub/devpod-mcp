use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::devpod;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodStatusParams {
    #[schemars(description = "Workspace name or ID")]
    workspace: String,
    #[serde(default)]
    #[schemars(description = "Timeout for status check, e.g. '30s'")]
    timeout: Option<String>,
}

#[tool_router(router = devpod_status_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devpod_status",
        description = "Get the status of a DevPod workspace. Returns structured JSON with state (Running, Stopped, Busy, NotFound)."
    )]
    async fn devpod_status(&self, Parameters(params): Parameters<DevpodStatusParams>) -> String {
        match devpod::status(&params.workspace, params.timeout.as_deref()).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
