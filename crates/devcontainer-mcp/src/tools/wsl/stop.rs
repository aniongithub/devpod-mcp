use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::wsl;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct WslTerminateParams {
    #[schemars(description = "WSL distribution name to terminate")]
    distro: String,
}

#[tool_router(router = wsl_stop_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "wsl_terminate",
        description = "Terminate (stop) a running WSL distribution."
    )]
    async fn wsl_terminate(&self, Parameters(params): Parameters<WslTerminateParams>) -> String {
        match wsl::terminate(&params.distro).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
