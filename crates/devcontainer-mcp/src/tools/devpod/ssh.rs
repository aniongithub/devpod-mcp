use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::devpod;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodSshParams {
    #[schemars(description = "Workspace name or ID")]
    workspace: String,
    #[schemars(description = "Command to execute inside the workspace")]
    command: String,
    #[serde(default)]
    #[schemars(description = "User to run the command as")]
    user: Option<String>,
    #[serde(default)]
    #[schemars(description = "Working directory inside the workspace")]
    workdir: Option<String>,
}

#[tool_router(router = devpod_ssh_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devpod_ssh",
        description = "Execute a command inside a DevPod workspace via SSH. Returns stdout, stderr, and exit code."
    )]
    async fn devpod_ssh(&self, Parameters(params): Parameters<DevpodSshParams>) -> String {
        match devpod::ssh_exec(
            &params.workspace,
            &params.command,
            params.user.as_deref(),
            params.workdir.as_deref(),
        )
        .await
        {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
