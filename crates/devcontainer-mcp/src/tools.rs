use rmcp::model::ServerInfo;
use rmcp::{tool, ServerHandler};

use devcontainer_mcp_core::{auth, cli::CliOutput, codespaces, devcontainer, devpod, docker};

#[derive(Debug, Clone)]
pub struct DevContainerMcp;

impl DevContainerMcp {
    pub fn new() -> Self {
        Self
    }
}

/// Helper: format a CliOutput as a JSON string for MCP responses.
fn format_output(output: &CliOutput) -> String {
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

    // =======================================================================
    // devcontainer CLI tools
    // =======================================================================

    #[tool(
        name = "devcontainer_up",
        description = "Create and start a local dev container using the devcontainer CLI. Requires a workspace folder with a devcontainer.json."
    )]
    async fn devcontainer_up(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Path to the workspace folder containing .devcontainer/devcontainer.json"
        )]
        workspace_folder: String,
        #[tool(param)]
        #[schemars(
            description = "Path to a specific devcontainer.json (overrides auto-detection)"
        )]
        config: Option<String>,
        #[tool(param)]
        #[schemars(
            description = "Additional flags as space-separated args, e.g. '--remove-existing-container --build-no-cache'"
        )]
        extra_args: Option<String>,
    ) -> String {
        let extra: Vec<&str> = extra_args
            .as_deref()
            .map(|a| a.split_whitespace().collect())
            .unwrap_or_default();
        match devcontainer::up(&workspace_folder, config.as_deref(), &extra).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devcontainer_exec",
        description = "Execute a command inside a running local dev container."
    )]
    async fn devcontainer_exec(
        &self,
        #[tool(param)]
        #[schemars(description = "Path to the workspace folder")]
        workspace_folder: String,
        #[tool(param)]
        #[schemars(description = "Command to execute inside the container")]
        command: String,
        #[tool(param)]
        #[schemars(description = "Arguments for the command as a space-separated string")]
        args: Option<String>,
    ) -> String {
        let cmd_args: Vec<&str> = args
            .as_deref()
            .map(|a| a.split_whitespace().collect())
            .unwrap_or_default();
        match devcontainer::exec(&workspace_folder, &command, &cmd_args).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devcontainer_build",
        description = "Build a dev container image without starting it."
    )]
    async fn devcontainer_build(
        &self,
        #[tool(param)]
        #[schemars(description = "Path to the workspace folder")]
        workspace_folder: String,
        #[tool(param)]
        #[schemars(
            description = "Additional flags as space-separated args, e.g. '--no-cache --image-name my-image'"
        )]
        extra_args: Option<String>,
    ) -> String {
        let extra: Vec<&str> = extra_args
            .as_deref()
            .map(|a| a.split_whitespace().collect())
            .unwrap_or_default();
        match devcontainer::build(&workspace_folder, &extra).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devcontainer_read_config",
        description = "Read and return the merged devcontainer configuration as JSON."
    )]
    async fn devcontainer_read_config(
        &self,
        #[tool(param)]
        #[schemars(description = "Path to the workspace folder")]
        workspace_folder: String,
        #[tool(param)]
        #[schemars(description = "Path to a specific devcontainer.json")]
        config: Option<String>,
    ) -> String {
        match devcontainer::read_configuration(&workspace_folder, config.as_deref()).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devcontainer_stop",
        description = "Stop a running local dev container (via Docker). The devcontainer CLI has no stop command, so this uses the Docker API directly."
    )]
    async fn devcontainer_stop(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Path to the workspace folder (used to find the container by label)"
        )]
        workspace_folder: String,
    ) -> String {
        match devcontainer::stop(&workspace_folder).await {
            Ok(msg) => msg,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devcontainer_remove",
        description = "Remove a local dev container and its resources (via Docker). Stops the container first if running."
    )]
    async fn devcontainer_remove(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Path to the workspace folder (used to find the container by label)"
        )]
        workspace_folder: String,
        #[tool(param)]
        #[schemars(description = "Force removal even if the container is running")]
        force: Option<bool>,
    ) -> String {
        match devcontainer::remove(&workspace_folder, force.unwrap_or(false)).await {
            Ok(msg) => msg,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devcontainer_status",
        description = "Get the status of a local dev container. Returns container info (state, image, labels) or null if not found."
    )]
    async fn devcontainer_status(
        &self,
        #[tool(param)]
        #[schemars(description = "Path to the workspace folder")]
        workspace_folder: String,
    ) -> String {
        match devcontainer::status(&workspace_folder).await {
            Ok(Some(info)) => {
                serde_json::to_string(&info).unwrap_or_else(|e| format!("Error: {e}"))
            }
            Ok(None) => r#"{"state":"NotFound"}"#.to_string(),
            Err(e) => format!("Error: {e}"),
        }
    }

    // =======================================================================
    // Auth tools
    // =======================================================================

    #[tool(
        name = "auth_status",
        description = "Check authentication status for a provider. Returns available auth handles and account info. Providers: 'github', 'aws', 'azure', 'gcloud', 'kubernetes'."
    )]
    async fn auth_status(
        &self,
        #[tool(param)]
        #[schemars(description = "Auth provider name (e.g. 'github', 'aws', 'azure', 'gcloud')")]
        provider: String,
    ) -> String {
        match auth::get_provider(&provider) {
            Some(p) => match p.status().await {
                Ok(status) => {
                    serde_json::to_string(&status).unwrap_or_else(|e| format!("Error: {e}"))
                }
                Err(e) => format!("Error: {e}"),
            },
            None => format!("Unknown auth provider: {provider}"),
        }
    }

    #[tool(
        name = "auth_login",
        description = "Initiate authentication for a provider. Opens browser, copies device code to clipboard, and waits for approval. Returns an auth handle on success."
    )]
    async fn auth_login(
        &self,
        #[tool(param)]
        #[schemars(description = "Auth provider name (e.g. 'github')")]
        provider: String,
        #[tool(param)]
        #[schemars(
            description = "Additional OAuth scopes to request (e.g. 'codespace' for GitHub)"
        )]
        scopes: Option<String>,
    ) -> String {
        match auth::get_provider(&provider) {
            Some(p) => match p.login(scopes.as_deref()).await {
                Ok(result) => {
                    serde_json::to_string(&result).unwrap_or_else(|e| format!("Error: {e}"))
                }
                Err(e) => format!("Error: {e}"),
            },
            None => format!("Unknown auth provider: {provider}"),
        }
    }

    #[tool(
        name = "auth_select",
        description = "Switch the active account for a provider. Returns account info if successful, null if the handle is invalid."
    )]
    async fn auth_select(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Auth handle to switch to (e.g. 'github-aniongithub', 'aws-prod')"
        )]
        id: String,
    ) -> String {
        let provider_name = auth::provider_from_handle(&id).unwrap_or("unknown");
        match auth::get_provider(provider_name) {
            Some(p) => match p.select(&id).await {
                Ok(Some(account)) => {
                    serde_json::to_string(&account).unwrap_or_else(|e| format!("Error: {e}"))
                }
                Ok(None) => format!("Failed to switch to: {id}"),
                Err(e) => format!("Error: {e}"),
            },
            None => format!("Unknown auth provider in handle: {id}"),
        }
    }

    #[tool(
        name = "auth_logout",
        description = "Logout / revoke an authenticated account. Removes credentials from the provider's keyring."
    )]
    async fn auth_logout(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Auth handle to logout (e.g. 'github-aniongithub', 'azure-<sub-id>')"
        )]
        id: String,
    ) -> String {
        let provider_name = auth::provider_from_handle(&id).unwrap_or("unknown");
        match auth::get_provider(provider_name) {
            Some(p) => match p.logout(&id).await {
                Ok(msg) => msg,
                Err(e) => format!("Error: {e}"),
            },
            None => format!("Unknown auth provider in handle: {id}"),
        }
    }

    // =======================================================================
    // GitHub Codespaces tools
    // =======================================================================

    #[tool(
        name = "codespaces_create",
        description = "Create a new GitHub Codespace for a repository. Requires a GitHub auth handle (get one via auth_status or auth_login)."
    )]
    #[allow(clippy::too_many_arguments)]
    async fn codespaces_create(
        &self,
        #[tool(param)]
        #[schemars(
            description = "GitHub auth handle from auth_status/auth_login (e.g. 'github-aniongithub')"
        )]
        auth: String,
        #[tool(param)]
        #[schemars(description = "Repository in owner/repo format")]
        repo: String,
        #[tool(param)]
        #[schemars(description = "Branch to create the codespace from")]
        branch: Option<String>,
        #[tool(param)]
        #[schemars(
            description = "Machine type — ask the user. Options: 'basicLinux32gb' (2 cores, 8 GB RAM), 'standardLinux32gb' (4 cores, 16 GB RAM), 'premiumLinux' (8 cores, 32 GB RAM), 'largePremiumLinux' (16 cores, 64 GB RAM)"
        )]
        machine: Option<String>,
        #[tool(param)]
        #[schemars(description = "Path to devcontainer.json within the repo")]
        devcontainer_path: Option<String>,
        #[tool(param)]
        #[schemars(description = "Display name for the codespace (max 48 chars)")]
        display_name: Option<String>,
        #[tool(param)]
        #[schemars(description = "Idle timeout before auto-stop, e.g. '10m', '1h'")]
        idle_timeout: Option<String>,
    ) -> String {
        let env = match auth::resolve_handle_env(&auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        match codespaces::create(
            &env,
            &repo,
            branch.as_deref(),
            machine.as_deref(),
            devcontainer_path.as_deref(),
            display_name.as_deref(),
            idle_timeout.as_deref(),
        )
        .await
        {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "codespaces_list",
        description = "List your GitHub Codespaces. Requires a GitHub auth handle."
    )]
    async fn codespaces_list(
        &self,
        #[tool(param)]
        #[schemars(description = "GitHub auth handle (e.g. 'github-aniongithub')")]
        auth: String,
        #[tool(param)]
        #[schemars(description = "Filter by repository (owner/repo format)")]
        repo: Option<String>,
    ) -> String {
        let env = match auth::resolve_handle_env(&auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        match codespaces::list(&env, repo.as_deref()).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "codespaces_ssh",
        description = "Execute a command inside a GitHub Codespace via SSH. Requires a GitHub auth handle."
    )]
    async fn codespaces_ssh(
        &self,
        #[tool(param)]
        #[schemars(description = "GitHub auth handle (e.g. 'github-aniongithub')")]
        auth: String,
        #[tool(param)]
        #[schemars(description = "Codespace name (from codespaces_list or codespaces_create)")]
        codespace: String,
        #[tool(param)]
        #[schemars(description = "Command to execute inside the codespace")]
        command: String,
    ) -> String {
        let env = match auth::resolve_handle_env(&auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        match codespaces::ssh_exec(&env, &codespace, &command).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "codespaces_stop",
        description = "Stop a running GitHub Codespace. Requires a GitHub auth handle."
    )]
    async fn codespaces_stop(
        &self,
        #[tool(param)]
        #[schemars(description = "GitHub auth handle (e.g. 'github-aniongithub')")]
        auth: String,
        #[tool(param)]
        #[schemars(description = "Codespace name")]
        codespace: String,
    ) -> String {
        let env = match auth::resolve_handle_env(&auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        match codespaces::stop(&env, &codespace).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "codespaces_delete",
        description = "Delete a GitHub Codespace. Requires a GitHub auth handle."
    )]
    async fn codespaces_delete(
        &self,
        #[tool(param)]
        #[schemars(description = "GitHub auth handle (e.g. 'github-aniongithub')")]
        auth: String,
        #[tool(param)]
        #[schemars(description = "Codespace name")]
        codespace: String,
        #[tool(param)]
        #[schemars(description = "Force delete even with unsaved changes")]
        force: Option<bool>,
    ) -> String {
        let env = match auth::resolve_handle_env(&auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        match codespaces::delete(&env, &codespace, force.unwrap_or(false)).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "codespaces_view",
        description = "View detailed information about a GitHub Codespace. Requires a GitHub auth handle."
    )]
    async fn codespaces_view(
        &self,
        #[tool(param)]
        #[schemars(description = "GitHub auth handle (e.g. 'github-aniongithub')")]
        auth: String,
        #[tool(param)]
        #[schemars(description = "Codespace name")]
        codespace: String,
    ) -> String {
        let env = match auth::resolve_handle_env(&auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        match codespaces::view(&env, &codespace).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "codespaces_ports",
        description = "List forwarded ports for a GitHub Codespace. Requires a GitHub auth handle."
    )]
    async fn codespaces_ports(
        &self,
        #[tool(param)]
        #[schemars(description = "GitHub auth handle (e.g. 'github-aniongithub')")]
        auth: String,
        #[tool(param)]
        #[schemars(description = "Codespace name")]
        codespace: String,
    ) -> String {
        let env = match auth::resolve_handle_env(&auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        match codespaces::ports(&env, &codespace).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}

#[tool(tool_box)]
impl ServerHandler for DevContainerMcp {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "DevContainer MCP — a unified MCP server for managing dev containers across \
                 multiple backends. Supports DevPod (devpod_* tools), the devcontainer CLI \
                 (devcontainer_* tools), and GitHub Codespaces (codespaces_* tools). \
                 Use the appropriate tool prefix based on the backend you want to use."
                    .into(),
            ),
            server_info: rmcp::model::Implementation {
                name: "devcontainer-mcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
            ..Default::default()
        }
    }
}
