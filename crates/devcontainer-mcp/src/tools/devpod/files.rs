use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::{devpod, file_ops};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodFileReadParams {
    #[schemars(description = "Workspace name or ID")]
    workspace: String,
    #[schemars(description = "Path to the file inside the workspace")]
    path: String,
    #[serde(default)]
    #[schemars(description = "Start line number (1-based, inclusive)")]
    start_line: Option<usize>,
    #[serde(default)]
    #[schemars(
        description = "End line number (1-based, inclusive). Use -1 or omit for end of file"
    )]
    end_line: Option<i64>,
    #[serde(default)]
    #[schemars(description = "User to run the command as")]
    user: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodFileWriteParams {
    #[schemars(description = "Workspace name or ID")]
    workspace: String,
    #[schemars(description = "Path to the file inside the workspace")]
    path: String,
    #[schemars(description = "File content to write")]
    content: String,
    #[serde(default)]
    #[schemars(description = "User to run the command as")]
    user: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodFileEditParams {
    #[schemars(description = "Workspace name or ID")]
    workspace: String,
    #[schemars(description = "Path to the file inside the workspace")]
    path: String,
    #[schemars(description = "The exact string in the file to replace. Must match exactly once.")]
    old_str: String,
    #[schemars(description = "The new string to replace old_str with")]
    new_str: String,
    #[serde(default)]
    #[schemars(description = "User to run the command as")]
    user: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodFileListParams {
    #[schemars(description = "Workspace name or ID")]
    workspace: String,
    #[serde(default)]
    #[schemars(description = "Path to the directory inside the workspace (defaults to '.')")]
    path: Option<String>,
    #[serde(default)]
    #[schemars(description = "User to run the command as")]
    user: Option<String>,
}

#[tool_router(router = devpod_files_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devpod_file_read",
        description = "Read file content from a DevPod workspace. Returns content with line numbers. Supports optional line range."
    )]
    async fn devpod_file_read(
        &self,
        Parameters(params): Parameters<DevpodFileReadParams>,
    ) -> String {
        match devpod::file_read(&params.workspace, &params.path, params.user.as_deref()).await {
            Ok(output) => {
                if output.exit_code != 0 {
                    return format!(
                        "Error (exit {}): {}",
                        output.exit_code,
                        output.stderr.trim()
                    );
                }
                let end = params
                    .end_line
                    .and_then(|e| if e < 0 { None } else { Some(e as usize) });
                file_ops::format_with_line_numbers(&output.stdout, params.start_line, end)
            }
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devpod_file_write",
        description = "Create or overwrite a file in a DevPod workspace. Creates parent directories automatically."
    )]
    async fn devpod_file_write(
        &self,
        Parameters(params): Parameters<DevpodFileWriteParams>,
    ) -> String {
        match devpod::file_write(
            &params.workspace,
            &params.path,
            &params.content,
            params.user.as_deref(),
        )
        .await
        {
            Ok(output) => {
                if output.exit_code != 0 {
                    format!(
                        "Error (exit {}): {}",
                        output.exit_code,
                        output.stderr.trim()
                    )
                } else {
                    format!("File written: {}", params.path)
                }
            }
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devpod_file_edit",
        description = "Make a surgical edit to a file in a DevPod workspace. Replaces exactly one occurrence of old_str with new_str. The old_str must match exactly one location in the file — include enough surrounding context to make it unique."
    )]
    async fn devpod_file_edit(
        &self,
        Parameters(params): Parameters<DevpodFileEditParams>,
    ) -> String {
        match devpod::file_edit(
            &params.workspace,
            &params.path,
            &params.old_str,
            &params.new_str,
            params.user.as_deref(),
        )
        .await
        {
            Ok(msg) => msg,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devpod_file_list",
        description = "List directory contents in a DevPod workspace. Shows non-hidden files up to 2 levels deep."
    )]
    async fn devpod_file_list(
        &self,
        Parameters(params): Parameters<DevpodFileListParams>,
    ) -> String {
        let dir = params.path.as_deref().unwrap_or(".");
        match devpod::file_list(&params.workspace, dir, params.user.as_deref()).await {
            Ok(output) => {
                if output.exit_code != 0 {
                    format!(
                        "Error (exit {}): {}",
                        output.exit_code,
                        output.stderr.trim()
                    )
                } else {
                    output.stdout
                }
            }
            Err(e) => format!("Error: {e}"),
        }
    }
}
