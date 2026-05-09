//! WSL (Windows Subsystem for Linux) backend.
//!
//! Wraps the `wsl.exe` CLI to manage WSL distributions, execute commands,
//! and perform file operations inside Linux distros.

use serde::Serialize;

use crate::cli::{run_cli, CliBinary, CliOutput};
use crate::error::{Error, Result};

/// A WSL distribution parsed from `wsl --list --verbose`.
#[derive(Debug, Clone, Serialize)]
pub struct WslDistro {
    pub name: String,
    pub state: String,
    pub version: u8,
    pub is_default: bool,
}

/// Run a WSL CLI command with the given args.
async fn run_wsl(args: &[&str], parse_json: bool) -> Result<CliOutput> {
    run_cli(&CliBinary::Wsl, args, parse_json).await
}

// ---------------------------------------------------------------------------
// Distribution management
// ---------------------------------------------------------------------------

/// `wsl --list --verbose` — list installed distributions with state and version.
///
/// Parses the tabular output into structured [`WslDistro`] entries and returns
/// them as a JSON array in `CliOutput::json`.
pub async fn list() -> Result<CliOutput> {
    let mut output = run_wsl(&["--list", "--verbose"], false).await?;

    if output.exit_code == 0 {
        let distros = parse_list_output(&output.stdout);
        output.json = Some(serde_json::to_value(&distros).unwrap_or_default());
    }

    Ok(output)
}

/// Parse the tabular output of `wsl --list --verbose`.
///
/// Example output:
/// ```text
///   NAME      STATE           VERSION
/// * Ubuntu    Running         2
///   Debian    Stopped         2
/// ```
fn parse_list_output(stdout: &str) -> Vec<WslDistro> {
    let mut distros = Vec::new();

    for line in stdout.lines().skip(1) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let is_default = trimmed.starts_with('*');
        let clean = trimmed.trim_start_matches('*').trim();

        let parts: Vec<&str> = clean.split_whitespace().collect();
        if parts.len() >= 3 {
            if let Ok(version) = parts[parts.len() - 1].parse::<u8>() {
                let state = parts[parts.len() - 2].to_string();
                let name = parts[..parts.len() - 2].join(" ");
                distros.push(WslDistro {
                    name,
                    state,
                    version,
                    is_default,
                });
            }
        }
    }

    distros
}

/// `wsl --set-default <distro>` — set the default WSL distribution.
pub async fn set_default(distro: &str) -> Result<CliOutput> {
    run_wsl(&["--set-default", distro], false).await
}

/// `wsl --terminate <distro>` — stop a running distribution.
pub async fn terminate(distro: &str) -> Result<CliOutput> {
    run_wsl(&["--terminate", distro], false).await
}

/// `wsl --shutdown` — shut down all running WSL distributions.
pub async fn shutdown() -> Result<CliOutput> {
    run_wsl(&["--shutdown"], false).await
}

// ---------------------------------------------------------------------------
// Command execution
// ---------------------------------------------------------------------------

/// `wsl -d <distro> -- sh -c <command>` — execute a command inside a distro.
pub async fn exec(distro: &str, command: &str) -> Result<CliOutput> {
    run_wsl(&["-d", distro, "--", "sh", "-c", command], false).await
}

// ---------------------------------------------------------------------------
// File operations
// ---------------------------------------------------------------------------

/// Read a file from a WSL distro.
pub async fn file_read(distro: &str, path: &str) -> Result<CliOutput> {
    let cmd = crate::file_ops::read_file_command(path);
    exec(distro, &cmd).await
}

/// Write (create or overwrite) a file in a WSL distro.
pub async fn file_write(distro: &str, path: &str, content: &str) -> Result<CliOutput> {
    let cmd = crate::file_ops::write_file_command(path, content);
    exec(distro, &cmd).await
}

/// Surgical edit: replace exactly one occurrence of `old_str` with `new_str`.
pub async fn file_edit(distro: &str, path: &str, old_str: &str, new_str: &str) -> Result<String> {
    let read_output = file_read(distro, path).await?;
    if read_output.exit_code != 0 {
        return Err(Error::FileRead(format!(
            "Failed to read {path}: {}",
            read_output.stderr.trim()
        )));
    }

    let modified = crate::file_ops::apply_edit(&read_output.stdout, old_str, new_str)?;

    let write_output = file_write(distro, path, &modified).await?;
    if write_output.exit_code != 0 {
        return Err(Error::FileEdit(format!(
            "Failed to write {path}: {}",
            write_output.stderr.trim()
        )));
    }

    Ok(format!("Edit applied to {path}"))
}

/// List directory contents in a WSL distro.
pub async fn file_list(distro: &str, path: &str) -> Result<CliOutput> {
    let cmd = crate::file_ops::list_dir_command(path);
    exec(distro, &cmd).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_list_output_typical() {
        let output = "\
  NAME      STATE           VERSION
* Ubuntu    Running         2
  Debian    Stopped         2
";
        let distros = parse_list_output(output);
        assert_eq!(distros.len(), 2);

        assert_eq!(distros[0].name, "Ubuntu");
        assert_eq!(distros[0].state, "Running");
        assert_eq!(distros[0].version, 2);
        assert!(distros[0].is_default);

        assert_eq!(distros[1].name, "Debian");
        assert_eq!(distros[1].state, "Stopped");
        assert_eq!(distros[1].version, 2);
        assert!(!distros[1].is_default);
    }

    #[test]
    fn test_parse_list_output_empty() {
        let output = "  NAME      STATE           VERSION\n";
        let distros = parse_list_output(output);
        assert!(distros.is_empty());
    }
}
