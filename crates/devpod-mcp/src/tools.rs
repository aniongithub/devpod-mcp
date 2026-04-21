use rmcp::model::ServerInfo;
use rmcp::{tool, ServerHandler};

use devpod_mcp_core::{devpod, docker};

#[derive(Debug, Clone)]
pub struct DevContainerMcp;

impl DevContainerMcp {
    pub fn new() -> Self {
        Self
    }
}

/// Helper: format a DevPodOutput as a JSON string for MCP responses.
fn format_output(output: &devpod::DevPodOutput) -> String {
    serde_json::json!({
        "exit_code": output.exit_code,
        "stdout": output.stdout,
        "stderr": output.stderr,
        "json": output.json,
    })
    .to_string()
}

#[tool(tool_box)]
impl DevContainerMcp {
    // -----------------------------------------------------------------------
    // Workspace lifecycle
    // -----------------------------------------------------------------------

    #[tool(
        name = "devpod_up",
        description = "Create and start a DevPod workspace. Pass the source (git URL, local path, or image) and any flags as space-separated args. Returns full build output for self-healing."
    )]
    async fn up(
        &self,
        #[tool(param)]
        #[schemars(
            description = "All arguments for 'devpod up', e.g. 'https://github.com/org/repo --provider docker --id my-ws'"
        )]
        args: String,
    ) -> String {
        let parts: Vec<&str> = args.split_whitespace().collect();
        match devpod::up(&parts).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(name = "devpod_stop", description = "Stop a running DevPod workspace.")]
    async fn stop(
        &self,
        #[tool(param)]
        #[schemars(description = "Workspace name or ID")]
        workspace: String,
    ) -> String {
        match devpod::stop(&workspace).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devpod_delete",
        description = "Delete a DevPod workspace. Stops and removes all associated resources."
    )]
    async fn delete(
        &self,
        #[tool(param)]
        #[schemars(description = "Workspace name or ID")]
        workspace: String,
        #[tool(param)]
        #[schemars(description = "Force delete even if workspace is not found remotely")]
        force: Option<bool>,
    ) -> String {
        match devpod::delete(&workspace, force.unwrap_or(false)).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devpod_build",
        description = "Build a DevPod workspace image without starting it."
    )]
    async fn build(
        &self,
        #[tool(param)]
        #[schemars(
            description = "All arguments for 'devpod build', e.g. 'my-workspace --provider docker'"
        )]
        args: String,
    ) -> String {
        let parts: Vec<&str> = args.split_whitespace().collect();
        match devpod::build(&parts).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    // -----------------------------------------------------------------------
    // Workspace queries
    // -----------------------------------------------------------------------

    #[tool(
        name = "devpod_status",
        description = "Get the status of a DevPod workspace. Returns structured JSON with state (Running, Stopped, Busy, NotFound)."
    )]
    async fn status(
        &self,
        #[tool(param)]
        #[schemars(description = "Workspace name or ID")]
        workspace: String,
        #[tool(param)]
        #[schemars(description = "Timeout for status check, e.g. '30s'")]
        timeout: Option<String>,
    ) -> String {
        match devpod::status(&workspace, timeout.as_deref()).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devpod_list",
        description = "List all DevPod workspaces. Returns JSON array with workspace IDs, sources, providers, and status."
    )]
    async fn list(&self) -> String {
        match devpod::list().await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    // -----------------------------------------------------------------------
    // Command execution
    // -----------------------------------------------------------------------

    #[tool(
        name = "devpod_ssh",
        description = "Execute a command inside a DevPod workspace via SSH. Returns stdout, stderr, and exit code."
    )]
    async fn ssh(
        &self,
        #[tool(param)]
        #[schemars(description = "Workspace name or ID")]
        workspace: String,
        #[tool(param)]
        #[schemars(description = "Command to execute inside the workspace")]
        command: String,
        #[tool(param)]
        #[schemars(description = "User to run the command as")]
        user: Option<String>,
        #[tool(param)]
        #[schemars(description = "Working directory inside the workspace")]
        workdir: Option<String>,
    ) -> String {
        match devpod::ssh_exec(&workspace, &command, user.as_deref(), workdir.as_deref()).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    // -----------------------------------------------------------------------
    // Logs
    // -----------------------------------------------------------------------

    #[tool(
        name = "devpod_logs",
        description = "Get logs from a DevPod workspace."
    )]
    async fn logs(
        &self,
        #[tool(param)]
        #[schemars(description = "Workspace name or ID")]
        workspace: String,
    ) -> String {
        match devpod::logs(&workspace).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    // -----------------------------------------------------------------------
    // Provider management
    // -----------------------------------------------------------------------

    #[tool(
        name = "devpod_provider_list",
        description = "List all configured DevPod providers."
    )]
    async fn provider_list(&self) -> String {
        match devpod::provider_list().await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(name = "devpod_provider_add", description = "Add a DevPod provider.")]
    async fn provider_add(
        &self,
        #[tool(param)]
        #[schemars(description = "Provider name or URL to add")]
        provider: String,
        #[tool(param)]
        #[schemars(description = "Additional options as space-separated KEY=VALUE pairs")]
        options: Option<String>,
    ) -> String {
        let opt_parts: Vec<&str> = options
            .as_deref()
            .map(|o| o.split_whitespace().collect())
            .unwrap_or_default();
        match devpod::provider_add(&provider, &opt_parts).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devpod_provider_delete",
        description = "Delete a DevPod provider."
    )]
    async fn provider_delete(
        &self,
        #[tool(param)]
        #[schemars(description = "Provider name to delete")]
        provider: String,
    ) -> String {
        match devpod::provider_delete(&provider).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    // -----------------------------------------------------------------------
    // Context management
    // -----------------------------------------------------------------------

    #[tool(
        name = "devpod_context_list",
        description = "List all DevPod contexts."
    )]
    async fn context_list(&self) -> String {
        match devpod::context_list().await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devpod_context_use",
        description = "Switch to a different DevPod context."
    )]
    async fn context_use(
        &self,
        #[tool(param)]
        #[schemars(description = "Context name to switch to")]
        context: String,
    ) -> String {
        match devpod::context_use(&context).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    // -----------------------------------------------------------------------
    // Direct Docker (via bollard)
    // -----------------------------------------------------------------------

    #[tool(
        name = "devpod_container_inspect",
        description = "Inspect a Docker container directly — returns labels, ports, mounts, and state. Useful for details DevPod CLI doesn't expose."
    )]
    async fn container_inspect(
        &self,
        #[tool(param)]
        #[schemars(description = "Container name or ID")]
        container: String,
    ) -> String {
        let client = match docker::connect() {
            Ok(c) => c,
            Err(e) => return format!("Error connecting to Docker: {e}"),
        };
        match docker::inspect_container(&client, &container).await {
            Ok(info) => serde_json::to_string(&info).unwrap_or_else(|e| format!("Error: {e}")),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devpod_container_logs",
        description = "Get Docker container logs directly via the Docker API. Supports tail parameter for last N lines."
    )]
    async fn container_logs(
        &self,
        #[tool(param)]
        #[schemars(description = "Container name or ID")]
        container: String,
        #[tool(param)]
        #[schemars(description = "Number of lines from the end to return (0 = all)")]
        tail: Option<usize>,
    ) -> String {
        let client = match docker::connect() {
            Ok(c) => c,
            Err(e) => return format!("Error connecting to Docker: {e}"),
        };
        match docker::container_logs(&client, &container, tail.unwrap_or(100)).await {
            Ok(logs) => logs,
            Err(e) => format!("Error: {e}"),
        }
    }
}

#[tool(tool_box)]
impl ServerHandler for DevContainerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "DevContainer MCP — wraps the DevPod CLI to give AI agents full control over \
                 isolated development environments. Use devpod_list to see workspaces, devpod_up \
                 to create one, devpod_ssh to run commands, devpod_stop/devpod_delete for lifecycle."
                    .into(),
            ),
            server_info: rmcp::model::Implementation {
                name: "devpod-mcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            ..Default::default()
        }
    }
}
