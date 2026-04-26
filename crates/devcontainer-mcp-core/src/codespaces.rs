use std::collections::HashMap;

use crate::cli::{run_cli_with_env, CliBinary, CliOutput};
use crate::error::Result;

const LIST_FIELDS: &str =
    "name,displayName,state,repository,gitStatus,createdAt,lastUsedAt,machineName";
const VIEW_FIELDS: &str = "name,displayName,state,owner,location,repository,gitStatus,devcontainerPath,machineName,machineDisplayName,prebuild,createdAt,lastUsedAt,idleTimeoutMinutes,retentionPeriodDays";
const PORT_FIELDS: &str = "sourcePort,visibility,label,browseUrl";

/// Run a `gh codespace` subcommand with auth env vars.
async fn run_gh_cs(
    args: &[&str],
    parse_json: bool,
    env: Option<&HashMap<String, String>>,
) -> Result<CliOutput> {
    let mut full_args = vec!["codespace"];
    full_args.extend_from_slice(args);
    run_cli_with_env(&CliBinary::Gh, &full_args, parse_json, env).await
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// `gh codespace create` — create a new codespace.
pub async fn create(
    env: &HashMap<String, String>,
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
    run_gh_cs(&args, false, Some(env)).await
}

/// `gh codespace list` — list codespaces.
pub async fn list(env: &HashMap<String, String>, repo: Option<&str>) -> Result<CliOutput> {
    let mut args = vec!["list", "--json", LIST_FIELDS];
    if let Some(r) = repo {
        args.push("--repo");
        args.push(r);
    }
    run_gh_cs(&args, true, Some(env)).await
}

/// `gh codespace ssh` — execute a command in a codespace.
pub async fn ssh_exec(
    env: &HashMap<String, String>,
    codespace: &str,
    command: &str,
) -> Result<CliOutput> {
    let args = vec!["ssh", "-c", codespace, "--", command];
    run_gh_cs(&args, false, Some(env)).await
}

/// `gh codespace stop` — stop a running codespace.
pub async fn stop(env: &HashMap<String, String>, codespace: &str) -> Result<CliOutput> {
    let args = vec!["stop", "-c", codespace];
    run_gh_cs(&args, false, Some(env)).await
}

/// `gh codespace delete` — delete a codespace.
pub async fn delete(
    env: &HashMap<String, String>,
    codespace: &str,
    force: bool,
) -> Result<CliOutput> {
    let mut args = vec!["delete", "-c", codespace];
    if force {
        args.push("--force");
    }
    run_gh_cs(&args, false, Some(env)).await
}

/// `gh codespace view` — view codespace details as JSON.
pub async fn view(env: &HashMap<String, String>, codespace: &str) -> Result<CliOutput> {
    let args = vec!["view", "-c", codespace, "--json", VIEW_FIELDS];
    run_gh_cs(&args, true, Some(env)).await
}

/// `gh codespace ports` — list forwarded ports as JSON.
pub async fn ports(env: &HashMap<String, String>, codespace: &str) -> Result<CliOutput> {
    let args = vec!["ports", "-c", codespace, "--json", PORT_FIELDS];
    run_gh_cs(&args, true, Some(env)).await
}

// ---------------------------------------------------------------------------
// File operations
// ---------------------------------------------------------------------------

/// Read a file from a Codespace.
pub async fn file_read(
    env: &HashMap<String, String>,
    codespace: &str,
    path: &str,
) -> Result<CliOutput> {
    let cmd = crate::file_ops::read_file_command(path);
    ssh_exec(env, codespace, &cmd).await
}

/// Write (create or overwrite) a file in a Codespace.
pub async fn file_write(
    env: &HashMap<String, String>,
    codespace: &str,
    path: &str,
    content: &str,
) -> Result<CliOutput> {
    let cmd = crate::file_ops::write_file_command(path, content);
    ssh_exec(env, codespace, &cmd).await
}

/// Surgical edit: replace exactly one occurrence of `old_str` with `new_str`.
pub async fn file_edit(
    env: &HashMap<String, String>,
    codespace: &str,
    path: &str,
    old_str: &str,
    new_str: &str,
) -> Result<String> {
    let read_output = file_read(env, codespace, path).await?;
    if read_output.exit_code != 0 {
        return Err(crate::error::Error::FileRead(format!(
            "Failed to read {path}: {}",
            read_output.stderr.trim()
        )));
    }

    let modified = crate::file_ops::apply_edit(&read_output.stdout, old_str, new_str)?;

    let write_output = file_write(env, codespace, path, &modified).await?;
    if write_output.exit_code != 0 {
        return Err(crate::error::Error::FileEdit(format!(
            "Failed to write {path}: {}",
            write_output.stderr.trim()
        )));
    }

    Ok(format!("Edit applied to {path}"))
}

/// List directory contents in a Codespace.
pub async fn file_list(
    env: &HashMap<String, String>,
    codespace: &str,
    path: &str,
) -> Result<CliOutput> {
    let cmd = crate::file_ops::list_dir_command(path);
    ssh_exec(env, codespace, &cmd).await
}
