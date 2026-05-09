use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::devpod;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodBuildParams {
    #[schemars(
        description = "All arguments for 'devpod build', e.g. 'my-workspace --provider docker'"
    )]
    args: String,
}

#[tool_router(router = devpod_build_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devpod_build",
        description = "Build a DevPod workspace image without starting it."
    )]
    async fn devpod_build(&self, Parameters(params): Parameters<DevpodBuildParams>) -> String {
        let parts: Vec<&str> = params.args.split_whitespace().collect();
        match devpod::build(&parts).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
