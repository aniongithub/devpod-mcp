use std::fs;
use std::path::PathBuf;

const FRONTMATTER_NAME: &str = "devcontainer-mcp";
const FRONTMATTER_DESC: &str =
    "Manage dev container environments via MCP tools (DevPod, devcontainer CLI, Codespaces)";

/// Fragment order for the assembled SKILL.md body.
const FRAGMENTS: &[&str] = &[
    "header.md",
    "core-rule.md",
    "auth.md",
    "choosing-backend.md",
    "devpod.md",
    "devcontainer.md",
    "codespaces.md",
    // WSL fragment inserted here on Windows builds
    "self-healing.md",
    "footer.md",
    "file-ops.md",
];

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("could not resolve workspace root from CARGO_MANIFEST_DIR");

    let skills_dir = workspace_root.join("skills");
    let tools_dir = skills_dir.join("_tools");
    let output_path = workspace_root.join("SKILL.md");

    // Use CARGO_CFG_TARGET_OS (the *target* platform, not the host) so that
    // cross-compiling for Windows from Linux still includes WSL content.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let is_windows_target = target_os == "windows";

    // --- Collect tool names from _tools/*.txt -----------------------------------
    let mut tools: Vec<String> = Vec::new();
    let mut tool_files: Vec<PathBuf> = fs::read_dir(&tools_dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", tools_dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "txt"))
        .collect();
    tool_files.sort();

    // On non-Windows targets, skip wsl.txt
    for path in &tool_files {
        let is_wsl = path
            .file_stem()
            .is_some_and(|s| s.to_str().is_some_and(|s| s == "wsl"));
        if is_wsl && !is_windows_target {
            continue;
        }
        let content = fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                tools.push(trimmed.to_string());
            }
        }
    }

    // --- Build YAML frontmatter -------------------------------------------------
    let mut output = String::from("---\n");
    output.push_str(&format!("name: {FRONTMATTER_NAME}\n"));
    output.push_str(&format!("description: {FRONTMATTER_DESC}\n"));
    output.push_str("tools:\n");
    for tool in &tools {
        output.push_str(&format!("  - {tool}\n"));
    }
    output.push_str("---\n");

    // --- Assemble markdown body -------------------------------------------------
    let insert_wsl_after = "codespaces.md";

    for &fragment_name in FRAGMENTS {
        let path = skills_dir.join(fragment_name);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        output.push('\n');
        output.push_str(&content);

        // On Windows targets, insert WSL section right after codespaces
        if is_windows_target && fragment_name == insert_wsl_after {
            let wsl_path = skills_dir.join("wsl.md");
            if wsl_path.exists() {
                let wsl_content = fs::read_to_string(&wsl_path)
                    .unwrap_or_else(|e| panic!("cannot read {}: {e}", wsl_path.display()));
                output.push('\n');
                output.push_str(&wsl_content);
            }
        }
    }

    // --- Write output -----------------------------------------------------------
    fs::write(&output_path, &output)
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", output_path.display()));

    // --- Incremental build support ----------------------------------------------
    println!("cargo:rerun-if-changed={}", skills_dir.display());
    println!("cargo:rerun-if-env-changed=CARGO_CFG_TARGET_OS");
}
