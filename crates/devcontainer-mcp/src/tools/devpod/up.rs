use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::devpod;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodUpParams {
    #[schemars(
        description = "All arguments for 'devpod up', e.g. 'https://github.com/org/repo --provider docker --id my-ws'"
    )]
    args: String,
}

#[tool_router(router = devpod_up_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devpod_up",
        description = "Create and start a DevPod workspace. Pass the source (git URL, local path, or image) and any flags as space-separated args. Returns full build output for self-healing."
    )]
    async fn devpod_up(&self, Parameters(params): Parameters<DevpodUpParams>) -> String {
        let parts: Vec<&str> = params.args.split_whitespace().collect();
        match devpod::up(&parts).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
