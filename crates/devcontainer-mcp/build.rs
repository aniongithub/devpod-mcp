use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const FRONTMATTER_NAME: &str = "devcontainer-mcp";
const FRONTMATTER_DESC: &str =
    "Manage dev container environments via MCP tools (DevPod, devcontainer CLI, Codespaces)";

/// Resolve the set of active tags for the current build target.
///
/// SKILL.md is read from the repo by all platforms, so we activate all tags
/// to ensure every tool is documented. The actual tool registration is
/// controlled by #[cfg(target_os)] in Rust source — build.rs only assembles
/// the documentation.
fn active_tags() -> HashSet<String> {
    let mut tags = HashSet::new();
    tags.insert("core".to_string());
    tags.insert("linux".to_string());
    tags.insert("macos".to_string());
    tags.insert("windows".to_string());
    tags.insert("docker-desktop".to_string());
    tags.insert("wsl".to_string());
    tags
}

/// Parse YAML frontmatter from a file's content.
/// Returns (tags, order, body) where body is everything after the closing `---`.
/// If no frontmatter is found, returns empty tags, order 0, and the full content.
fn parse_frontmatter(content: &str) -> (Vec<String>, i64, &str) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (vec![], 0, content);
    }

    // Find the closing ---
    let after_open = &trimmed[3..];
    let close_pos = match after_open.find("\n---") {
        Some(pos) => pos,
        None => return (vec![], 0, content),
    };

    let frontmatter = &after_open[..close_pos];
    let body_start = 3 + close_pos + 4; // "---" + frontmatter + "\n---"
    let body = trimmed[body_start..].trim_start_matches('\n');

    let mut tags = vec![];
    let mut order: i64 = 0;

    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("tags:") {
            // Parse [tag1, tag2] or tag1, tag2
            let rest = rest.trim().trim_start_matches('[').trim_end_matches(']');
            for tag in rest.split(',') {
                let tag = tag.trim();
                if !tag.is_empty() {
                    tags.push(tag.to_string());
                }
            }
        } else if let Some(rest) = line.strip_prefix("order:") {
            order = rest.trim().parse().unwrap_or(0);
        }
    }

    (tags, order, body)
}

/// Check if a fragment's required tags are all present in the active set.
/// Empty tags means always included.
fn tags_match(required: &[String], active: &HashSet<String>) -> bool {
    required.iter().all(|tag| active.contains(tag))
}

/// Discover and sort all .md files in a directory, filtering by active tags.
fn collect_fragments(dir: &Path, active: &HashSet<String>) -> Vec<(i64, String)> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "md"))
        .collect();
    entries.sort();

    let mut fragments: Vec<(i64, String)> = Vec::new();

    for path in entries {
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let (tags, order, body) = parse_frontmatter(&content);

        if tags_match(&tags, active) {
            fragments.push((order, body.to_string()));
        }
    }

    fragments.sort_by_key(|(order, _)| *order);
    fragments
}

/// Discover all .txt tool lists, filtering by active tags.
fn collect_tools(dir: &Path, active: &HashSet<String>) -> Vec<String> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "txt"))
        .collect();
    entries.sort();

    let mut tools = Vec::new();

    for path in entries {
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        let (tags, _, body) = parse_frontmatter(&content);

        if tags_match(&tags, active) {
            for line in body.lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    tools.push(trimmed.to_string());
                }
            }
        }
    }

    tools
}

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("could not resolve workspace root from CARGO_MANIFEST_DIR");

    let skills_dir = workspace_root.join("skills");
    let tools_dir = skills_dir.join("_tools");
    let output_path = workspace_root.join("SKILL.md");

    let active = active_tags();

    // --- Collect tool names -----------------------------------------------------
    let tools = collect_tools(&tools_dir, &active);

    // --- Build YAML frontmatter -------------------------------------------------
    let mut output = String::from("---\n");
    output.push_str(&format!("name: {FRONTMATTER_NAME}\n"));
    output.push_str(&format!("description: {FRONTMATTER_DESC}\n"));
    output.push_str("tools:\n");
    for tool in &tools {
        output.push_str(&format!("  - {tool}\n"));
    }
    output.push_str("---\n");

    // --- Assemble markdown body (ordered by frontmatter `order` field) -----------
    let fragments = collect_fragments(&skills_dir, &active);
    for (_, body) in &fragments {
        output.push('\n');
        output.push_str(body);
    }

    // --- Write output -----------------------------------------------------------
    fs::write(&output_path, &output)
        .unwrap_or_else(|e| panic!("cannot write {}: {e}", output_path.display()));

    // --- Incremental build support ----------------------------------------------
    println!("cargo:rerun-if-changed={}", skills_dir.display());
}
