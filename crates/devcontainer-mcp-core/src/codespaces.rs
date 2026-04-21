use crate::cli::{run_cli, CliBinary, CliOutput};
use crate::error::Result;

const LIST_FIELDS: &str =
    "name,displayName,state,repository,gitStatus,createdAt,lastUsedAt,machineName";
const VIEW_FIELDS: &str = "name,displayName,state,owner,location,repository,gitStatus,devcontainerPath,machineName,machineDisplayName,prebuild,createdAt,lastUsedAt,idleTimeoutMinutes,retentionPeriodDays";
const PORT_FIELDS: &str = "sourcePort,visibility,label,browseUrl";

/// Run a `gh codespace` subcommand.
async fn run_gh_cs(args: &[&str], parse_json: bool) -> Result<CliOutput> {
    let mut full_args = vec!["codespace"];
    full_args.extend_from_slice(args);
    run_cli(&CliBinary::Gh, &full_args, parse_json).await
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
