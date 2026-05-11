//! Integration tests for the devcontainer-guard.sh hook script.
//!
//! These tests verify that the hook correctly blocks/allows tool calls
//! based on the agent format, tool name, devcontainer presence, and bypass.

use std::process::Command;

fn repo_root() -> std::path::PathBuf {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("could not find repo root")
        .to_path_buf()
}

fn hook_path() -> std::path::PathBuf {
    repo_root().join("hooks/devcontainer-guard.sh")
}

/// Run the hook script with the given JSON input and return (stdout, exit_code).
fn run_hook(json_input: &str) -> (String, i32) {
    let output = Command::new("bash")
        .arg(hook_path())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .take()
                .unwrap()
                .write_all(json_input.as_bytes())
                .unwrap();
            child.wait_with_output()
        })
        .expect("failed to run hook script");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let code = output.status.code().unwrap_or(-1);
    (stdout, code)
}

fn cwd_with_devcontainer() -> String {
    repo_root().to_string_lossy().to_string()
}

// -----------------------------------------------------------------------
// Copilot CLI format
// -----------------------------------------------------------------------

#[test]
fn copilot_cli_bash_with_devcontainer_denies() {
    let input = format!(
        r#"{{"toolName":"bash","toolArgs":{{"command":"cargo build"}},"cwd":"{}"}}"#,
        cwd_with_devcontainer()
    );
    let (stdout, code) = run_hook(&input);
    assert_eq!(code, 0);
    assert!(
        stdout.contains(r#""permissionDecision":"deny""#)
            || stdout.contains(r#""permissionDecision": "deny""#)
    );
    // Copilot CLI format should NOT have hookSpecificOutput
    assert!(!stdout.contains("hookSpecificOutput"));
}

#[test]
fn copilot_cli_shell_with_devcontainer_denies() {
    let input = format!(
        r#"{{"toolName":"shell","toolArgs":{{"command":"make"}},"cwd":"{}"}}"#,
        cwd_with_devcontainer()
    );
    let (stdout, _) = run_hook(&input);
    assert!(stdout.contains("deny"));
}

#[test]
fn copilot_cli_powershell_with_devcontainer_denies() {
    let input = format!(
        r#"{{"toolName":"powershell","toolArgs":{{"command":"dir"}},"cwd":"{}"}}"#,
        cwd_with_devcontainer()
    );
    let (stdout, _) = run_hook(&input);
    assert!(stdout.contains("deny"));
}

#[test]
fn copilot_cli_view_tool_allows() {
    let input = format!(
        r#"{{"toolName":"view","toolArgs":{{"path":"src/main.rs"}},"cwd":"{}"}}"#,
        cwd_with_devcontainer()
    );
    let (stdout, code) = run_hook(&input);
    assert_eq!(code, 0);
    assert!(stdout.is_empty(), "non-bash tool should produce no output");
}

#[test]
fn copilot_cli_no_devcontainer_allows() {
    let input = r#"{"toolName":"bash","toolArgs":{"command":"ls"},"cwd":"/tmp"}"#;
    let (stdout, code) = run_hook(input);
    assert_eq!(code, 0);
    assert!(stdout.is_empty());
}

// -----------------------------------------------------------------------
// Claude Code format
// -----------------------------------------------------------------------

#[test]
fn claude_code_bash_with_devcontainer_denies() {
    let input = format!(
        r#"{{"tool_name":"Bash","tool_input":{{"command":"npm install"}},"cwd":"{}"}}"#,
        cwd_with_devcontainer()
    );
    let (stdout, code) = run_hook(&input);
    assert_eq!(code, 0);
    assert!(stdout.contains("hookSpecificOutput"));
    assert!(stdout.contains("deny"));
}

#[test]
fn claude_code_edit_tool_allows() {
    let input = format!(
        r#"{{"tool_name":"Edit","tool_input":{{"path":"src/main.rs"}},"cwd":"{}"}}"#,
        cwd_with_devcontainer()
    );
    let (stdout, code) = run_hook(&input);
    assert_eq!(code, 0);
    assert!(stdout.is_empty());
}

// -----------------------------------------------------------------------
// Bypass
// -----------------------------------------------------------------------

#[test]
fn bypass_string_allows_through() {
    let input = format!(
        r#"{{"tool_name":"Bash","tool_input":{{"command":"USER_CONFIRMED_HOST_OPERATION=1 cargo build"}},"cwd":"{}"}}"#,
        cwd_with_devcontainer()
    );
    let (stdout, code) = run_hook(&input);
    assert_eq!(code, 0);
    assert!(stdout.is_empty(), "bypass should produce no output");
}

// -----------------------------------------------------------------------
// Edge cases
// -----------------------------------------------------------------------

#[test]
fn missing_cwd_allows() {
    let input = r#"{"toolName":"bash","toolArgs":{"command":"ls"}}"#;
    let (stdout, code) = run_hook(input);
    assert_eq!(code, 0);
    assert!(stdout.is_empty());
}

#[test]
fn empty_tool_name_allows() {
    let input = format!(
        r#"{{"toolName":"","toolArgs":{{}},"cwd":"{}"}}"#,
        cwd_with_devcontainer()
    );
    let (stdout, code) = run_hook(&input);
    assert_eq!(code, 0);
    assert!(stdout.is_empty());
}
