use devcontainer_mcp_core::cli::CliOutput;

/// Format a CliOutput as a JSON string for MCP responses.
pub fn format_output(output: &CliOutput) -> String {
    serde_json::json!({
        "exit_code": output.exit_code,
        "stdout": output.stdout,
        "stderr": output.stderr,
        "json": output.json,
    })
    .to_string()
}
