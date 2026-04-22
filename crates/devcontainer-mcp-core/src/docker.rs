use bollard::container::{
    ListContainersOptions, LogsOptions, RemoveContainerOptions, StopContainerOptions,
};
use bollard::Docker;
use futures_util::StreamExt;
use serde::Serialize;
use std::collections::HashMap;

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

/// Create a Docker client connected to the local socket.
pub fn connect() -> Result<Docker> {
    Ok(Docker::connect_with_local_defaults()?)
}

/// Find a container by the standard `devcontainer.local_folder` label.
pub async fn find_container_by_local_folder(
    docker: &Docker,
    local_folder: &str,
) -> Result<Option<ContainerInfo>> {
    let mut filters = HashMap::new();
    filters.insert(
        "label".to_string(),
        vec![format!("devcontainer.local_folder={local_folder}")],
    );

    let options = ListContainersOptions {
        all: true,
        filters,
        ..Default::default()
    };

    let containers = docker.list_containers(Some(options)).await?;

    Ok(containers.into_iter().next().map(|c| {
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
    }))
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
