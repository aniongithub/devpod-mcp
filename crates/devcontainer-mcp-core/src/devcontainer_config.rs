//! Discovery and parsing of `devcontainer.json` files within a workspace.
//!
//! Implements the VS Code multi-container workspace pattern: a single repo
//! can contain several `.devcontainer/<name>/devcontainer.json` files,
//! typically referencing different services of a shared `docker-compose.yml`.
//!
//! Used by the `devcontainer_list_configs` MCP tool and as the basis for
//! disambiguating Docker container lookups when an operation targets a
//! specific config in a multi-container workspace.

use crate::error::Result;
use jsonc_parser::ParseOptions;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// One field of a `dockerComposeFile` — either a single path or an array.
fn json_to_string_list(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        _ => Vec::new(),
    }
}

/// Classification of a parsed devcontainer.json based on which top-level
/// fields are present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigKind {
    /// `dockerComposeFile` + `service` — multi-container compose project.
    Compose,
    /// `build.dockerfile` or top-level `dockerFile` — image is built locally.
    Dockerfile,
    /// `image` — pre-built image is used directly.
    Image,
    /// None of the above could be determined.
    Unknown,
}

/// One entry returned by [`list_configs`]: a discovered devcontainer.json
/// with a best-effort summary of its top-level fields. `path` is always
/// **relative** to the workspace folder so it can be fed straight back
/// into another tool's `config` parameter.
#[derive(Debug, Clone, Serialize)]
pub struct DiscoveredConfig {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub docker_compose_file: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_folder: Option<String>,
    pub kind: ConfigKind,
    /// When parsing fails, the entry still appears with the error message
    /// here and all other fields empty — so an agent can see *that* the
    /// config exists even if it can't be parsed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Resolved view of a devcontainer.json used by the Docker container lookup
/// to disambiguate multi-container workspaces. Paths are absolute and
/// canonicalized so substring comparisons against container labels are
/// reliable across `/var/folders/...` ↔ `/private/var/folders/...` aliases
/// (macOS) and similar.
#[derive(Debug, Clone)]
pub struct ResolvedConfig {
    pub abs_path: PathBuf,
    pub service: Option<String>,
    /// Absolute, canonicalized paths of every `dockerComposeFile` entry.
    pub compose_files_abs: Vec<PathBuf>,
    pub kind: ConfigKind,
}

/// Parse `text` as JSONC (JSON with comments + trailing commas, per the
/// devcontainer spec) into a `serde_json::Value`.
fn parse_jsonc(text: &str) -> std::result::Result<serde_json::Value, String> {
    let parsed = jsonc_parser::parse_to_serde_value(text, &ParseOptions::default())
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "empty document".to_string())?;
    Ok(parsed)
}

/// Determine the search paths for devcontainer configs under `workspace`.
/// Returns absolute paths in priority order: root `.devcontainer.json`,
/// `.devcontainer/devcontainer.json`, then every
/// `.devcontainer/*/devcontainer.json`.
fn candidate_paths(workspace: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let root_dotfile = workspace.join(".devcontainer.json");
    if root_dotfile.is_file() {
        out.push(root_dotfile);
    }
    let dc_dir = workspace.join(".devcontainer");
    let root_in_dir = dc_dir.join("devcontainer.json");
    if root_in_dir.is_file() {
        out.push(root_in_dir);
    }
    if let Ok(entries) = std::fs::read_dir(&dc_dir) {
        // Sort sub-folder configs deterministically so the agent sees a
        // stable order across calls.
        let mut subdirs: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        subdirs.sort();
        for sub in subdirs {
            let candidate = sub.join("devcontainer.json");
            if candidate.is_file() {
                out.push(candidate);
            }
        }
    }
    out
}

/// Convert `abs` to a path relative to `base`, falling back to the absolute
/// path's string form if it isn't actually inside `base`.
fn rel_to(abs: &Path, base: &Path) -> String {
    abs.strip_prefix(base)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| abs.to_string_lossy().into_owned())
}

/// Best-effort canonicalization that falls back to the input on error.
/// Used because compose files referenced from a not-yet-built devcontainer
/// might not exist at scan time, but we still want a stable absolute path.
fn canonicalize_or_keep(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

fn classify(obj: &serde_json::Map<String, serde_json::Value>) -> ConfigKind {
    if obj.get("dockerComposeFile").is_some() {
        return ConfigKind::Compose;
    }
    let has_dockerfile = obj.get("dockerFile").is_some()
        || obj
            .get("build")
            .and_then(|b| b.as_object())
            .is_some_and(|b| b.contains_key("dockerfile") || b.contains_key("dockerFile"));
    if has_dockerfile {
        return ConfigKind::Dockerfile;
    }
    if obj.get("image").is_some() {
        return ConfigKind::Image;
    }
    ConfigKind::Unknown
}

/// Build a [`DiscoveredConfig`] from a parsed JSON value. Top-level keys are
/// best-effort: missing or wrong-typed fields are skipped, not errors.
fn build_discovered(rel_path: String, value: &serde_json::Value) -> DiscoveredConfig {
    let Some(obj) = value.as_object() else {
        return DiscoveredConfig {
            path: rel_path,
            error: Some("top-level value is not an object".into()),
            kind: ConfigKind::Unknown,
            name: None,
            image: None,
            service: None,
            docker_compose_file: Vec::new(),
            workspace_folder: None,
        };
    };
    DiscoveredConfig {
        path: rel_path,
        name: obj.get("name").and_then(|v| v.as_str()).map(str::to_string),
        image: obj
            .get("image")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        service: obj
            .get("service")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        docker_compose_file: obj
            .get("dockerComposeFile")
            .map(json_to_string_list)
            .unwrap_or_default(),
        workspace_folder: obj
            .get("workspaceFolder")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        kind: classify(obj),
        error: None,
    }
}

/// Discover every devcontainer.json under `workspace_folder` and return a
/// best-effort summary of each. Parse errors are reported per-entry; the
/// call itself only fails on IO problems with `workspace_folder`.
pub fn list_configs(workspace_folder: &str) -> Result<Vec<DiscoveredConfig>> {
    let workspace = PathBuf::from(workspace_folder);
    let workspace_abs = canonicalize_or_keep(&workspace);
    let mut out = Vec::new();
    for abs_path in candidate_paths(&workspace_abs) {
        let rel = rel_to(&abs_path, &workspace_abs);
        let entry = match std::fs::read_to_string(&abs_path) {
            Ok(text) => match parse_jsonc(&text) {
                Ok(v) => build_discovered(rel, &v),
                Err(e) => DiscoveredConfig {
                    path: rel,
                    error: Some(format!("JSONC parse error: {e}")),
                    kind: ConfigKind::Unknown,
                    name: None,
                    image: None,
                    service: None,
                    docker_compose_file: Vec::new(),
                    workspace_folder: None,
                },
            },
            Err(e) => DiscoveredConfig {
                path: rel,
                error: Some(format!("read error: {e}")),
                kind: ConfigKind::Unknown,
                name: None,
                image: None,
                service: None,
                docker_compose_file: Vec::new(),
                workspace_folder: None,
            },
        };
        out.push(entry);
    }
    Ok(out)
}

/// Resolve `config` (a path to a devcontainer.json, either absolute or
/// relative to `workspace_folder`) into a [`ResolvedConfig`] used by the
/// Docker container lookup.
///
/// `dockerComposeFile` entries are resolved against the *config file's*
/// directory (matching the devcontainer CLI's behavior) and canonicalized.
pub fn resolve_config(workspace_folder: &str, config: &str) -> Result<ResolvedConfig> {
    let workspace = canonicalize_or_keep(&PathBuf::from(workspace_folder));
    let config_path = {
        let p = PathBuf::from(config);
        if p.is_absolute() {
            p
        } else {
            workspace.join(p)
        }
    };
    let abs_path = canonicalize_or_keep(&config_path);
    let text = std::fs::read_to_string(&abs_path)?;
    let value = parse_jsonc(&text).map_err(|e| {
        crate::error::Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("JSONC parse error in {}: {e}", abs_path.display()),
        ))
    })?;
    let obj = value.as_object().ok_or_else(|| {
        crate::error::Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("top-level value is not an object in {}", abs_path.display()),
        ))
    })?;

    let kind = classify(obj);
    let service = obj
        .get("service")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    // `dockerComposeFile` paths are resolved against the directory
    // containing the devcontainer.json, per the devcontainer spec.
    let config_dir = abs_path.parent().unwrap_or(&workspace).to_path_buf();
    let compose_files_abs: Vec<PathBuf> = obj
        .get("dockerComposeFile")
        .map(json_to_string_list)
        .unwrap_or_default()
        .into_iter()
        .map(|entry| {
            let p = PathBuf::from(&entry);
            let abs = if p.is_absolute() {
                p
            } else {
                config_dir.join(p)
            };
            canonicalize_or_keep(&abs)
        })
        .collect();

    Ok(ResolvedConfig {
        abs_path,
        service,
        compose_files_abs,
        kind,
    })
}

/// Helper for callers that want a `BTreeMap<String, String>` of common
/// label filters derived from a [`ResolvedConfig`]. Not used directly by
/// the docker layer (which builds its own filters), but handy for tests
/// and tracing.
pub fn expected_labels(resolved: &ResolvedConfig) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if let Some(svc) = &resolved.service {
        out.insert("com.docker.compose.service".into(), svc.clone());
    }
    out.insert(
        "devcontainer.config_file".into(),
        resolved.abs_path.to_string_lossy().into_owned(),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write(p: &Path, contents: &str) {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, contents).unwrap();
    }

    #[test]
    fn discovers_root_devcontainer_in_subdir() {
        let dir = tempdir().unwrap();
        write(
            &dir.path().join(".devcontainer/devcontainer.json"),
            r#"{ "name": "Root", "image": "alpine:3.20" }"#,
        );
        let out = list_configs(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].path, ".devcontainer/devcontainer.json");
        assert_eq!(out[0].name.as_deref(), Some("Root"));
        assert_eq!(out[0].kind, ConfigKind::Image);
    }

    #[test]
    fn discovers_root_dotfile() {
        let dir = tempdir().unwrap();
        write(
            &dir.path().join(".devcontainer.json"),
            r#"{ "image": "alpine:3.20" }"#,
        );
        let out = list_configs(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].path, ".devcontainer.json");
    }

    #[test]
    fn discovers_multi_subfolder_configs_sorted() {
        let dir = tempdir().unwrap();
        write(
            &dir.path().join(".devcontainer/python/devcontainer.json"),
            r#"// python config
            {
              "name": "Python",
              "dockerComposeFile": ["../../docker-compose.yml"],
              "service": "python-api",
              "workspaceFolder": "/workspace/py",
            }"#,
        );
        write(
            &dir.path().join(".devcontainer/node/devcontainer.json"),
            r#"{
              "name": "Node",
              "dockerComposeFile": "../../docker-compose.yml",
              "service": "node-app"
            }"#,
        );
        let out = list_configs(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(out.len(), 2);
        // Sorted by directory name: node, python
        assert_eq!(out[0].service.as_deref(), Some("node-app"));
        assert_eq!(out[1].service.as_deref(), Some("python-api"));
        assert_eq!(out[0].kind, ConfigKind::Compose);
        assert_eq!(out[1].kind, ConfigKind::Compose);
        // Single-string dockerComposeFile is normalized to a one-element list.
        assert_eq!(out[0].docker_compose_file.len(), 1);
        // Trailing comma + comments parsed by JSONC.
        assert_eq!(out[1].workspace_folder.as_deref(), Some("/workspace/py"));
    }

    #[test]
    fn malformed_config_reports_per_entry_error() {
        let dir = tempdir().unwrap();
        write(
            &dir.path().join(".devcontainer/broken/devcontainer.json"),
            r#"{ "name": "broken" "#,
        );
        let out = list_configs(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(out.len(), 1);
        assert!(out[0].error.is_some());
    }

    #[test]
    fn classifies_dockerfile_build() {
        let dir = tempdir().unwrap();
        write(
            &dir.path().join(".devcontainer/devcontainer.json"),
            r#"{ "build": { "dockerfile": "Dockerfile" } }"#,
        );
        let out = list_configs(dir.path().to_str().unwrap()).unwrap();
        assert_eq!(out[0].kind, ConfigKind::Dockerfile);
    }

    #[test]
    fn resolve_config_canonicalizes_compose_files() {
        let dir = tempdir().unwrap();
        write(
            &dir.path().join("docker-compose.yml"),
            "services:\n  a:\n    image: alpine\n",
        );
        write(
            &dir.path().join(".devcontainer/a/devcontainer.json"),
            r#"{
              "dockerComposeFile": ["../../docker-compose.yml"],
              "service": "a"
            }"#,
        );
        let resolved = resolve_config(
            dir.path().to_str().unwrap(),
            ".devcontainer/a/devcontainer.json",
        )
        .unwrap();
        assert_eq!(resolved.service.as_deref(), Some("a"));
        assert_eq!(resolved.kind, ConfigKind::Compose);
        assert_eq!(resolved.compose_files_abs.len(), 1);
        let compose = &resolved.compose_files_abs[0];
        assert!(compose.is_absolute());
        assert!(compose.ends_with("docker-compose.yml"));
    }

    #[test]
    fn list_returns_empty_when_no_configs() {
        let dir = tempdir().unwrap();
        let out = list_configs(dir.path().to_str().unwrap()).unwrap();
        assert!(out.is_empty());
    }
}
