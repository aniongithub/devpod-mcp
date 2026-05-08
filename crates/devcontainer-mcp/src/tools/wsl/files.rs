use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::{file_ops, wsl};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct WslFileReadParams {
    #[schemars(description = "WSL distribution name")]
    distro: String,
    #[schemars(description = "Path to the file inside the distribution")]
    path: String,
    #[serde(default)]
    #[schemars(description = "Start line number (1-based, inclusive)")]
    start_line: Option<usize>,
    #[serde(default)]
    #[schemars(
        description = "End line number (1-based, inclusive). Use -1 or omit for end of file"
    )]
    end_line: Option<i64>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct WslFileWriteParams {
    #[schemars(description = "WSL distribution name")]
    distro: String,
    #[schemars(description = "Path to the file inside the distribution")]
    path: String,
    #[schemars(description = "File content to write")]
    content: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct WslFileEditParams {
    #[schemars(description = "WSL distribution name")]
    distro: String,
    #[schemars(description = "Path to the file inside the distribution")]
    path: String,
    #[schemars(description = "The exact string in the file to replace. Must match exactly once.")]
    old_str: String,
    #[schemars(description = "The new string to replace old_str with")]
    new_str: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct WslFileListParams {
    #[schemars(description = "WSL distribution name")]
    distro: String,
    #[serde(default)]
    #[schemars(description = "Path to the directory inside the distribution (defaults to '.')")]
    path: Option<String>,
}

#[tool_router(router = wsl_files_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "wsl_file_read",
        description = "Read file content from a WSL distribution. Returns content with line numbers. Supports optional line range."
    )]
    async fn wsl_file_read(&self, Parameters(params): Parameters<WslFileReadParams>) -> String {
        match wsl::file_read(&params.distro, &params.path).await {
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
        name = "wsl_file_write",
        description = "Create or overwrite a file in a WSL distribution. Creates parent directories automatically."
    )]
    async fn wsl_file_write(&self, Parameters(params): Parameters<WslFileWriteParams>) -> String {
        match wsl::file_write(&params.distro, &params.path, &params.content).await {
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
        name = "wsl_file_edit",
        description = "Make a surgical edit to a file in a WSL distribution. Replaces exactly one occurrence of old_str with new_str. The old_str must match exactly one location in the file — include enough surrounding context to make it unique."
    )]
    async fn wsl_file_edit(&self, Parameters(params): Parameters<WslFileEditParams>) -> String {
        match wsl::file_edit(
            &params.distro,
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
        name = "wsl_file_list",
        description = "List directory contents in a WSL distribution. Shows non-hidden files up to 2 levels deep."
    )]
    async fn wsl_file_list(&self, Parameters(params): Parameters<WslFileListParams>) -> String {
        let dir = params.path.as_deref().unwrap_or(".");
        match wsl::file_list(&params.distro, dir).await {
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
