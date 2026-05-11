use devcontainer_mcp_core::devcontainer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerExecParams {
    #[schemars(description = "Path to the workspace folder")]
    workspace_folder: String,
    #[schemars(description = "Command to execute inside the container")]
    command: String,
    #[schemars(description = "Arguments for the command as a space-separated string")]
    args: Option<String>,
}

#[tool_router(router = devcontainer_exec_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devcontainer_exec",
        description = "Execute a command inside a running local dev container."
    )]
    async fn devcontainer_exec(
        &self,
        Parameters(params): Parameters<DevcontainerExecParams>,
    ) -> String {
        let full_cmd = match &params.args {
            Some(a) => format!("{} {}", params.command, a),
            None => params.command,
        };
        match devcontainer::exec(&params.workspace_folder, "sh", &["-c", &full_cmd]).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
