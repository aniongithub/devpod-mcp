use std::process::Stdio;
use tokio::process::Command as TokioCommand;

use crate::cli::{run_cli, CliBinary, CliOutput};
use crate::error::Result;

const LIST_FIELDS: &str =
    "name,displayName,state,repository,gitStatus,createdAt,lastUsedAt,machineName";
const VIEW_FIELDS: &str = "name,displayName,state,owner,location,repository,gitStatus,devcontainerPath,machineName,machineDisplayName,prebuild,createdAt,lastUsedAt,idleTimeoutMinutes,retentionPeriodDays";
const PORT_FIELDS: &str = "sourcePort,visibility,label,browseUrl";

/// Detect whether a CLI output indicates a missing OAuth scope.
fn needs_auth_scope(output: &CliOutput) -> Option<String> {
    let combined = format!("{}{}", output.stdout, output.stderr);
    if combined.contains("gh auth refresh") || combined.contains("gh auth login") {
        // Extract the suggested scope from the error message
        // e.g. 'This API operation needs the "codespace" scope.'
        if let Some(start) = combined.find("needs the \"") {
            let rest = &combined[start + 11..];
            if let Some(end) = rest.find('"') {
                return Some(rest[..end].to_string());
            }
        }
        // Fallback: generic codespace scope
        Some("codespace".to_string())
    } else {
        None
    }
}

/// Run `gh auth refresh` with --clipboard to copy the device code,
/// then open the browser to the device auth page.
/// Returns a user-friendly message with instructions.
pub async fn request_auth_scope(scope: &str) -> Result<CliOutput> {
    // Run gh auth refresh with --clipboard to copy device code
    let auth_output = run_cli(
        &CliBinary::Gh,
        &[
            "auth",
            "refresh",
            "-h",
            "github.com",
            "-s",
            scope,
            "--clipboard",
        ],
        false,
    )
    .await?;

    // Try to open the browser to the device auth page
    let open_cmd = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = TokioCommand::new(open_cmd)
        .arg("https://github.com/login/device")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    Ok(auth_output)
}

/// Run a `gh codespace` subcommand. If the command fails due to a missing
/// OAuth scope, automatically trigger the device-code auth flow.
async fn run_gh_cs(args: &[&str], parse_json: bool) -> Result<CliOutput> {
    let mut full_args = vec!["codespace"];
    full_args.extend_from_slice(args);
    let output = run_cli(&CliBinary::Gh, &full_args, parse_json).await?;

    if output.exit_code != 0 {
        if let Some(scope) = needs_auth_scope(&output) {
            let auth_result = request_auth_scope(&scope).await?;
            let combined = format!("{}{}", auth_result.stdout, auth_result.stderr);

            // Extract the device code from output
            let code_hint = if let Some(pos) = combined.find("one-time code:") {
                let rest = &combined[pos..];
                rest.lines().next().unwrap_or("").to_string()
            } else {
                String::new()
            };

            return Ok(CliOutput {
                exit_code: 1,
                stdout: String::new(),
                stderr: String::new(),
                json: Some(serde_json::json!({
                    "auth_required": true,
                    "scope": scope,
                    "message": format!(
                        "GitHub auth scope '{}' required. Device code copied to clipboard. \
                         Approve in the browser that just opened, then retry the command.",
                        scope
                    ),
                    "detail": code_hint,
                    "browser_opened": "https://github.com/login/device",
                })),
            });
        }
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// `gh codespace create` — create a new codespace.
pub async fn create(
    repo: &str,
    branch: Option<&str>,
    machine: Option<&str>,
    devcontainer_path: Option<&str>,
    display_name: Option<&str>,
    idle_timeout: Option<&str>,
) -> Result<CliOutput> {
    let mut args = vec!["create", "--repo", repo];
    if let Some(b) = branch {
        args.push("--branch");
        args.push(b);
    }
    if let Some(m) = machine {
        args.push("--machine");
        args.push(m);
    }
    if let Some(d) = devcontainer_path {
        args.push("--devcontainer-path");
        args.push(d);
    }
    if let Some(n) = display_name {
        args.push("--display-name");
        args.push(n);
    }
    if let Some(t) = idle_timeout {
        args.push("--idle-timeout");
        args.push(t);
    }
    run_gh_cs(&args, false).await
}

/// `gh codespace list` — list codespaces.
pub async fn list(repo: Option<&str>) -> Result<CliOutput> {
    let mut args = vec!["list", "--json", LIST_FIELDS];
    if let Some(r) = repo {
        args.push("--repo");
        args.push(r);
    }
    run_gh_cs(&args, true).await
}

/// `gh codespace ssh` — execute a command in a codespace.
pub async fn ssh_exec(codespace: &str, command: &str) -> Result<CliOutput> {
    let args = vec!["ssh", "-c", codespace, "--", command];
    run_gh_cs(&args, false).await
}

/// `gh codespace stop` — stop a running codespace.
pub async fn stop(codespace: &str) -> Result<CliOutput> {
    let args = vec!["stop", "-c", codespace];
    run_gh_cs(&args, false).await
}

/// `gh codespace delete` — delete a codespace.
pub async fn delete(codespace: &str, force: bool) -> Result<CliOutput> {
    let mut args = vec!["delete", "-c", codespace];
    if force {
        args.push("--force");
    }
    run_gh_cs(&args, false).await
}

/// `gh codespace view` — view codespace details as JSON.
pub async fn view(codespace: &str) -> Result<CliOutput> {
    let args = vec!["view", "-c", codespace, "--json", VIEW_FIELDS];
    run_gh_cs(&args, true).await
}

/// `gh codespace ports` — list forwarded ports as JSON.
pub async fn ports(codespace: &str) -> Result<CliOutput> {
    let args = vec!["ports", "-c", codespace, "--json", PORT_FIELDS];
    run_gh_cs(&args, true).await
}
