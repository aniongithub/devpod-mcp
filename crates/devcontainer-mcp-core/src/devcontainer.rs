use crate::cli::{run_cli, CliBinary, CliOutput};
use crate::docker;
use crate::error::Result;

/// Run a `devcontainer` CLI command.
async fn run_devcontainer(args: &[&str], parse_json: bool) -> Result<CliOutput> {
    run_cli(&CliBinary::Devcontainer, args, parse_json).await
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// `devcontainer up` — create and start a dev container.
pub async fn up(
    workspace_folder: &str,
    config: Option<&str>,
    extra_args: &[&str],
) -> Result<CliOutput> {
    let mut args = vec!["up", "--workspace-folder", workspace_folder];
    if let Some(c) = config {
        args.push("--config");
        args.push(c);
    }
    args.extend_from_slice(extra_args);
    run_devcontainer(&args, true).await
}

/// `devcontainer exec` — execute a command in a running dev container.
pub async fn exec(
    workspace_folder: &str,
    command: &str,
    command_args: &[&str],
) -> Result<CliOutput> {
    let mut args = vec!["exec", "--workspace-folder", workspace_folder, command];
    args.extend_from_slice(command_args);
    run_devcontainer(&args, false).await
}

/// `devcontainer build` — build a dev container image.
pub async fn build(workspace_folder: &str, extra_args: &[&str]) -> Result<CliOutput> {
    let mut args = vec!["build", "--workspace-folder", workspace_folder];
    args.extend_from_slice(extra_args);
    run_devcontainer(&args, true).await
}

/// `devcontainer read-configuration` — read devcontainer config as JSON.
pub async fn read_configuration(workspace_folder: &str, config: Option<&str>) -> Result<CliOutput> {
    let mut args = vec!["read-configuration", "--workspace-folder", workspace_folder];
    if let Some(c) = config {
        args.push("--config");
        args.push(c);
    }
    args.push("--include-merged-configuration");
    run_devcontainer(&args, true).await
}

// ---------------------------------------------------------------------------
// Lifecycle via bollard (devcontainer CLI has no stop/down)
// ---------------------------------------------------------------------------

/// Stop a dev container found by its workspace folder label.
pub async fn stop(workspace_folder: &str) -> Result<String> {
    let client = docker::connect()?;
    let container = docker::find_container_by_local_folder(&client, workspace_folder)
        .await?
        .ok_or_else(|| {
            crate::error::Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("No devcontainer found for workspace: {workspace_folder}"),
            ))
        })?;
    docker::stop_container(&client, &container.id).await?;
    Ok(format!("Stopped container {}", container.name))
}

/// Remove a dev container found by its workspace folder label.
pub async fn remove(workspace_folder: &str, force: bool) -> Result<String> {
    let client = docker::connect()?;
    let container = docker::find_container_by_local_folder(&client, workspace_folder)
        .await?
        .ok_or_else(|| {
            crate::error::Error::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("No devcontainer found for workspace: {workspace_folder}"),
            ))
        })?;
    docker::remove_container(&client, &container.id, force).await?;
    Ok(format!("Removed container {}", container.name))
}

/// Get status of a dev container by workspace folder label.
pub async fn status(workspace_folder: &str) -> Result<Option<docker::ContainerInfo>> {
    let client = docker::connect()?;
    docker::find_container_by_local_folder(&client, workspace_folder).await
}

// ---------------------------------------------------------------------------
// File operations
// ---------------------------------------------------------------------------

/// Read a file from a dev container.
pub async fn file_read(
    workspace_folder: &str,
    path: &str,
) -> Result<CliOutput> {
    let cmd = crate::file_ops::read_file_command(path);
    exec(workspace_folder, "sh", &["-c", &cmd]).await
}

/// Write (create or overwrite) a file in a dev container.
pub async fn file_write(
    workspace_folder: &str,
    path: &str,
    content: &str,
) -> Result<CliOutput> {
    let cmd = crate::file_ops::write_file_command(path, content);
    exec(workspace_folder, "sh", &["-c", &cmd]).await
}

/// Surgical edit: replace exactly one occurrence of `old_str` with `new_str`.
pub async fn file_edit(
    workspace_folder: &str,
    path: &str,
    old_str: &str,
    new_str: &str,
) -> Result<String> {
    let read_output = file_read(workspace_folder, path).await?;
    if read_output.exit_code != 0 {
        return Err(crate::error::Error::FileRead(format!(
            "Failed to read {path}: {}",
            read_output.stderr.trim()
        )));
    }

    let modified = crate::file_ops::apply_edit(&read_output.stdout, old_str, new_str)?;

    let write_output = file_write(workspace_folder, path, &modified).await?;
    if write_output.exit_code != 0 {
        return Err(crate::error::Error::FileEdit(format!(
            "Failed to write {path}: {}",
            write_output.stderr.trim()
        )));
    }

    Ok(format!("Edit applied to {path}"))
}

/// List directory contents in a dev container.
pub async fn file_list(
    workspace_folder: &str,
    path: &str,
) -> Result<CliOutput> {
    let cmd = crate::file_ops::list_dir_command(path);
    exec(workspace_folder, "sh", &["-c", &cmd]).await
}
