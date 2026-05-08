use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::wsl;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct WslExecParams {
    #[schemars(description = "WSL distribution name")]
    distro: String,
    #[schemars(description = "Command to execute")]
    command: String,
}

#[tool_router(router = wsl_exec_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "wsl_exec",
        description = "Execute a command inside a WSL distribution. Returns stdout, stderr, and exit code."
    )]
    async fn wsl_exec(&self, Parameters(params): Parameters<WslExecParams>) -> String {
        match wsl::exec(&params.distro, &params.command).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
