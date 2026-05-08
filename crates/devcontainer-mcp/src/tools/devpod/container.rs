use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::docker;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodContainerInspectParams {
    #[schemars(description = "Container name or ID")]
    container: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodContainerLogsParams {
    #[schemars(description = "Container name or ID")]
    container: String,
    #[serde(default)]
    #[schemars(description = "Number of lines from the end to return (0 = all)")]
    tail: Option<usize>,
}

#[tool_router(router = devpod_container_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devpod_container_inspect",
        description = "Inspect a Docker container directly — returns labels, ports, mounts, and state. Useful for details DevPod CLI doesn't expose."
    )]
    async fn devpod_container_inspect(
        &self,
        Parameters(params): Parameters<DevpodContainerInspectParams>,
    ) -> String {
        let client = match docker::connect() {
            Ok(c) => c,
            Err(e) => return format!("Error connecting to Docker: {e}"),
        };
        match docker::inspect_container(&client, &params.container).await {
            Ok(info) => serde_json::to_string(&info).unwrap_or_else(|e| format!("Error: {e}")),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devpod_container_logs",
        description = "Get Docker container logs directly via the Docker API. Supports tail parameter for last N lines."
    )]
    async fn devpod_container_logs(
        &self,
        Parameters(params): Parameters<DevpodContainerLogsParams>,
    ) -> String {
        let client = match docker::connect() {
            Ok(c) => c,
            Err(e) => return format!("Error connecting to Docker: {e}"),
        };
        match docker::container_logs(&client, &params.container, params.tail.unwrap_or(100)).await {
            Ok(logs) => logs,
            Err(e) => format!("Error: {e}"),
        }
    }
}
