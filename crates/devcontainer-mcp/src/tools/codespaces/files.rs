use devcontainer_mcp_core::{auth, codespaces, file_ops};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CodespacesFileReadParams {
    #[schemars(description = "GitHub auth handle (e.g. 'github-aniongithub')")]
    auth: String,
    #[schemars(description = "Codespace name")]
    codespace: String,
    #[schemars(description = "Path to the file inside the codespace")]
    path: String,
    #[schemars(description = "Start line number (1-based, inclusive)")]
    start_line: Option<usize>,
    #[schemars(
        description = "End line number (1-based, inclusive). Use -1 or omit for end of file"
    )]
    end_line: Option<i64>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CodespacesFileWriteParams {
    #[schemars(description = "GitHub auth handle (e.g. 'github-aniongithub')")]
    auth: String,
    #[schemars(description = "Codespace name")]
    codespace: String,
    #[schemars(description = "Path to the file inside the codespace")]
    path: String,
    #[schemars(description = "File content to write")]
    content: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CodespacesFileEditParams {
    #[schemars(description = "GitHub auth handle (e.g. 'github-aniongithub')")]
    auth: String,
    #[schemars(description = "Codespace name")]
    codespace: String,
    #[schemars(description = "Path to the file inside the codespace")]
    path: String,
    #[schemars(description = "The exact string in the file to replace. Must match exactly once.")]
    old_str: String,
    #[schemars(description = "The new string to replace old_str with")]
    new_str: String,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct CodespacesFileListParams {
    #[schemars(description = "GitHub auth handle (e.g. 'github-aniongithub')")]
    auth: String,
    #[schemars(description = "Codespace name")]
    codespace: String,
    #[schemars(description = "Path to the directory inside the codespace (defaults to '.')")]
    path: Option<String>,
}

#[tool_router(router = codespaces_files_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "codespaces_file_read",
        description = "Read file content from a GitHub Codespace. Returns content with line numbers. Supports optional line range. Requires a GitHub auth handle."
    )]
    async fn codespaces_file_read(
        &self,
        Parameters(params): Parameters<CodespacesFileReadParams>,
    ) -> String {
        let env = match auth::resolve_handle_env(&params.auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        match codespaces::file_read(&env, &params.codespace, &params.path).await {
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
        name = "codespaces_file_write",
        description = "Create or overwrite a file in a GitHub Codespace. Creates parent directories automatically. Requires a GitHub auth handle."
    )]
    async fn codespaces_file_write(
        &self,
        Parameters(params): Parameters<CodespacesFileWriteParams>,
    ) -> String {
        let env = match auth::resolve_handle_env(&params.auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        match codespaces::file_write(&env, &params.codespace, &params.path, &params.content).await {
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
        name = "codespaces_file_edit",
        description = "Make a surgical edit to a file in a GitHub Codespace. Replaces exactly one occurrence of old_str with new_str. The old_str must match exactly one location in the file — include enough surrounding context to make it unique. Requires a GitHub auth handle."
    )]
    async fn codespaces_file_edit(
        &self,
        Parameters(params): Parameters<CodespacesFileEditParams>,
    ) -> String {
        let env = match auth::resolve_handle_env(&params.auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        match codespaces::file_edit(
            &env,
            &params.codespace,
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
        name = "codespaces_file_list",
        description = "List directory contents in a GitHub Codespace. Shows non-hidden files up to 2 levels deep. Requires a GitHub auth handle."
    )]
    async fn codespaces_file_list(
        &self,
        Parameters(params): Parameters<CodespacesFileListParams>,
    ) -> String {
        let env = match auth::resolve_handle_env(&params.auth).await {
            Ok(e) => e,
            Err(e) => return format!("Auth error: {e}"),
        };
        let dir = params.path.as_deref().unwrap_or(".");
        match codespaces::file_list(&env, &params.codespace, dir).await {
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
