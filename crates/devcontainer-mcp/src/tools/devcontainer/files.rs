use devcontainer_mcp_core::{devcontainer, file_ops};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerFileReadParams {
    #[schemars(description = "Path to the workspace folder")]
    workspace_folder: String,
    #[schemars(
        description = "Path to a specific devcontainer.json (use to disambiguate multi-container workspaces)"
    )]
    config: Option<String>,
    #[schemars(description = "Path to the file inside the container")]
    path: String,
    #[schemars(description = "Start line number (1-based, inclusive)")]
    start_line: Option<usize>,
    #[schemars(
        description = "End line number (1-based, inclusive). Use -1 or omit for end of file"
    )]
    end_line: Option<i64>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerFileWriteParams {
    #[schemars(description = "Path to the workspace folder")]
    workspace_folder: String,
    #[schemars(
        description = "Path to a specific devcontainer.json (use to disambiguate multi-container workspaces)"
    )]
    config: Option<String>,
    #[schemars(description = "Path to the file inside the container")]
    path: String,
    #[schemars(description = "File content to write")]
    content: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerFileEditParams {
    #[schemars(description = "Path to the workspace folder")]
    workspace_folder: String,
    #[schemars(
        description = "Path to a specific devcontainer.json (use to disambiguate multi-container workspaces)"
    )]
    config: Option<String>,
    #[schemars(description = "Path to the file inside the container")]
    path: String,
    #[schemars(description = "The exact string in the file to replace. Must match exactly once.")]
    old_str: String,
    #[schemars(description = "The new string to replace old_str with")]
    new_str: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerFileListParams {
    #[schemars(description = "Path to the workspace folder")]
    workspace_folder: String,
    #[schemars(
        description = "Path to a specific devcontainer.json (use to disambiguate multi-container workspaces)"
    )]
    config: Option<String>,
    #[schemars(description = "Path to the directory inside the container (defaults to '.')")]
    path: Option<String>,
}

#[tool_router(router = devcontainer_files_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devcontainer_file_read",
        description = "Read file content from a local dev container. Returns content with line numbers. Supports optional line range."
    )]
    async fn devcontainer_file_read(
        &self,
        Parameters(params): Parameters<DevcontainerFileReadParams>,
    ) -> String {
        match devcontainer::file_read(
            &params.workspace_folder,
            params.config.as_deref(),
            &params.path,
        )
        .await
        {
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
        name = "devcontainer_file_write",
        description = "Create or overwrite a file in a local dev container. Creates parent directories automatically."
    )]
    async fn devcontainer_file_write(
        &self,
        Parameters(params): Parameters<DevcontainerFileWriteParams>,
    ) -> String {
        match devcontainer::file_write(
            &params.workspace_folder,
            params.config.as_deref(),
            &params.path,
            &params.content,
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
        name = "devcontainer_file_edit",
        description = "Make a surgical edit to a file in a local dev container. Replaces exactly one occurrence of old_str with new_str. The old_str must match exactly one location in the file — include enough surrounding context to make it unique."
    )]
    async fn devcontainer_file_edit(
        &self,
        Parameters(params): Parameters<DevcontainerFileEditParams>,
    ) -> String {
        match devcontainer::file_edit(
            &params.workspace_folder,
            params.config.as_deref(),
            &params.path,
            &params.old_str,
            &params.new_str,
        )
        .await
        {
            Ok(msg) => msg,
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devcontainer_file_list",
        description = "List directory contents in a local dev container. Shows non-hidden files up to 2 levels deep."
    )]
    async fn devcontainer_file_list(
        &self,
        Parameters(params): Parameters<DevcontainerFileListParams>,
    ) -> String {
        let dir = params.path.as_deref().unwrap_or(".");
        match devcontainer::file_list(&params.workspace_folder, params.config.as_deref(), dir).await
        {
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
