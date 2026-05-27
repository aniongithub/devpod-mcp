use bollard::container::{
    ListContainersOptions, LogsOptions, RemoveContainerOptions, StopContainerOptions,
};
use bollard::Docker;
use futures_util::StreamExt;
use serde::Serialize;
use std::collections::HashMap;

use crate::devcontainer_config::{ConfigKind, ResolvedConfig};
use crate::error::Result;

/// Summary of a container's state.
#[derive(Debug, Clone, Serialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub labels: HashMap<String, String>,
}

impl ContainerInfo {
    /// `com.docker.compose.service` label, if present — used for ambiguity
    /// diagnostics in multi-container workspaces.
    pub fn compose_service(&self) -> Option<&str> {
        self.labels
            .get("com.docker.compose.service")
            .map(String::as_str)
    }

    /// `devcontainer.config_file` label, if present.
    pub fn devcontainer_config_file(&self) -> Option<&str> {
        self.labels
            .get("devcontainer.config_file")
            .map(String::as_str)
    }
}

/// Outcome of [`find_devcontainer`]. `Many` is surfaced to callers so they
/// can either pick deterministically (e.g. status returning the first +
/// flagging `multipleMatches`) or refuse and ask for a `config` (e.g. stop
/// / remove).
#[derive(Debug, Clone)]
pub enum DevcontainerLookup {
    None,
    One(ContainerInfo),
    Many(Vec<ContainerInfo>),
}

impl DevcontainerLookup {
    pub fn into_one(self) -> Option<ContainerInfo> {
        match self {
            DevcontainerLookup::One(c) => Some(c),
            _ => None,
        }
    }

    /// Convenience: collapse `One`/`Many` to "any of them" for diagnostics.
    pub fn candidates(&self) -> &[ContainerInfo] {
        match self {
            DevcontainerLookup::None => &[],
            DevcontainerLookup::One(c) => std::slice::from_ref(c),
            DevcontainerLookup::Many(v) => v.as_slice(),
        }
    }
}

/// Create a Docker client connected to the local socket.
pub fn connect() -> Result<Docker> {
    Ok(Docker::connect_with_local_defaults()?)
}

/// Internal helper: list containers matching one label filter.
async fn list_by_label(docker: &Docker, label_eq: &str) -> Result<Vec<ContainerInfo>> {
    let mut filters = HashMap::new();
    filters.insert("label".to_string(), vec![label_eq.to_string()]);
    let options = ListContainersOptions {
        all: true,
        filters,
        ..Default::default()
    };
    let containers = docker.list_containers(Some(options)).await?;
    Ok(containers.into_iter().map(container_to_info).collect())
}

fn container_to_info(c: bollard::models::ContainerSummary) -> ContainerInfo {
    let labels = c.labels.unwrap_or_default();
    ContainerInfo {
        id: c.id.unwrap_or_default(),
        name: c
            .names
            .and_then(|n| n.first().cloned())
            .unwrap_or_default()
            .trim_start_matches('/')
            .to_string(),
        image: c.image.unwrap_or_default(),
        state: c.state.unwrap_or_default(),
        labels,
    }
}

/// Find devcontainer-managed container(s) for `workspace_folder`, optionally
/// scoped to a specific resolved config.
///
/// Strategy:
///
/// 1. **Compose config provided** — filter by `com.docker.compose.service`
///    and verify the container's `com.docker.compose.project.config_files`
///    label contains one of the resolved compose-file paths. This is the
///    only reliable path for sibling containers in a multi-service
///    workspace, because the devcontainer CLI only stamps `devcontainer.*`
///    labels on the first container of the compose project.
/// 2. **Image / Dockerfile config provided** — filter by
///    `devcontainer.config_file=<abs config path>`. The CLI labels these
///    consistently.
/// 3. **No config** — start with `devcontainer.local_folder=<workspace>`.
///    If that misses, fall back to
///    `com.docker.compose.project.working_dir=<workspace>` to catch the
///    multi-container case where sibling containers lack devcontainer.*
///    labels.
pub async fn find_devcontainer(
    docker: &Docker,
    workspace_folder: &str,
    config: Option<&ResolvedConfig>,
) -> Result<DevcontainerLookup> {
    // Canonicalize the workspace path the same way the CLI does so
    // label substring/equality checks survive macOS `/private/var/...`
    // aliasing and trailing-slash variations.
    let workspace_abs = std::fs::canonicalize(workspace_folder)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| workspace_folder.to_string());

    let candidates: Vec<ContainerInfo> = match config {
        Some(cfg) if cfg.kind == ConfigKind::Compose => {
            if let Some(service) = cfg.service.as_deref() {
                let service_filter = format!("com.docker.compose.service={service}");
                let by_service = list_by_label(docker, &service_filter).await?;
                let expected: Vec<String> = cfg
                    .compose_files_abs
                    .iter()
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                by_service
                    .into_iter()
                    .filter(|c| {
                        let Some(cfg_files) =
                            c.labels.get("com.docker.compose.project.config_files")
                        else {
                            return false;
                        };
                        expected.iter().any(|exp| cfg_files.contains(exp.as_str()))
                    })
                    .collect()
            } else {
                // Compose config without a service field — fall back to
                // the no-config matching logic inline (avoids async
                // recursion).
                lookup_without_config(docker, &workspace_abs).await?
            }
        }
        Some(cfg) => {
            let cfg_str = cfg.abs_path.to_string_lossy();
            let filter = format!("devcontainer.config_file={cfg_str}");
            list_by_label(docker, &filter).await?
        }
        None => lookup_without_config(docker, &workspace_abs).await?,
    };

    Ok(match candidates.len() {
        0 => DevcontainerLookup::None,
        1 => DevcontainerLookup::One(candidates.into_iter().next().unwrap()),
        _ => DevcontainerLookup::Many(candidates),
    })
}

/// No-config matching: union of `devcontainer.local_folder` and
/// `com.docker.compose.project.working_dir` matches (deduped by container
/// id). Single-container workspaces yield exactly the primary container;
/// multi-container compose workspaces yield every sibling, even those
/// that don't carry `devcontainer.*` labels (the CLI only stamps them on
/// the first container of the compose project).
async fn lookup_without_config(docker: &Docker, workspace_abs: &str) -> Result<Vec<ContainerInfo>> {
    let primary = format!("devcontainer.local_folder={workspace_abs}");
    let mut hits = list_by_label(docker, &primary).await?;
    let fallback = format!("com.docker.compose.project.working_dir={workspace_abs}");
    let fallback_hits = list_by_label(docker, &fallback).await?;
    let seen: std::collections::HashSet<String> = hits.iter().map(|c| c.id.clone()).collect();
    for c in fallback_hits {
        if !seen.contains(&c.id) {
            hits.push(c);
        }
    }
    Ok(hits)
}

/// Inspect a container by name or ID.
pub async fn inspect_container(docker: &Docker, name_or_id: &str) -> Result<ContainerInfo> {
    let detail = docker.inspect_container(name_or_id, None).await?;

    let labels = detail
        .config
        .as_ref()
        .and_then(|c| c.labels.clone())
        .unwrap_or_default();

    let image = detail
        .config
        .as_ref()
        .and_then(|c| c.image.clone())
        .unwrap_or_default();

    let state_str = detail
        .state
        .as_ref()
        .and_then(|s| s.status.as_ref())
        .map(|s| format!("{s:?}").to_lowercase())
        .unwrap_or_else(|| "unknown".to_string());

    Ok(ContainerInfo {
        id: detail.id.unwrap_or_default(),
        name: detail
            .name
            .unwrap_or_default()
            .trim_start_matches('/')
            .to_string(),
        image,
        state: state_str,
        labels,
    })
}

/// Stream container logs, returning them as a single string.
/// `tail` limits to the last N lines (0 = all).
pub async fn container_logs(docker: &Docker, container_id: &str, tail: usize) -> Result<String> {
    let options = LogsOptions::<String> {
        stdout: true,
        stderr: true,
        tail: if tail > 0 {
            tail.to_string()
        } else {
            "all".to_string()
        },
        ..Default::default()
    };

    let mut stream = docker.logs(container_id, Some(options));
    let mut output = String::new();

    while let Some(msg) = stream.next().await {
        match msg {
            Ok(log) => output.push_str(&log.to_string()),
            Err(e) => return Err(e.into()),
        }
    }

    Ok(output)
}

/// Stop a container by name or ID.
pub async fn stop_container(docker: &Docker, name_or_id: &str) -> Result<()> {
    docker
        .stop_container(name_or_id, Some(StopContainerOptions { t: 10 }))
        .await?;
    Ok(())
}

/// Remove a container by name or ID.
pub async fn remove_container(docker: &Docker, name_or_id: &str, force: bool) -> Result<()> {
    docker
        .remove_container(
            name_or_id,
            Some(RemoveContainerOptions {
                force,
                ..Default::default()
            }),
        )
        .await?;
    Ok(())
}
