use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::wsl;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct WslSetDefaultParams {
    #[schemars(description = "WSL distribution name to set as default")]
    distro: String,
}

#[tool_router(router = wsl_set_default_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "wsl_set_default",
        description = "Set the default WSL distribution."
    )]
    async fn wsl_set_default(&self, Parameters(params): Parameters<WslSetDefaultParams>) -> String {
        match wsl::set_default(&params.distro).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
