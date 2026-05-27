use devcontainer_mcp_core::devcontainer::{self, StatusOutcome};
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};
use serde_json::json;

use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerStatusParams {
    #[schemars(description = "Path to the workspace folder")]
    workspace_folder: String,
    #[schemars(
        description = "Path to a specific devcontainer.json (use to disambiguate multi-container workspaces)"
    )]
    config: Option<String>,
}

#[tool_router(router = devcontainer_status_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devcontainer_status",
        description = "Get the status of a local dev container. Returns container info (state, image, labels) for a single match, `{\"state\":\"NotFound\"}` if nothing matches, or `{\"state\":\"Ambiguous\",\"candidates\":[...]}` if multiple containers match the workspace and no `config` was supplied to disambiguate."
    )]
    async fn devcontainer_status(
        &self,
        Parameters(params): Parameters<DevcontainerStatusParams>,
    ) -> String {
        match devcontainer::status(&params.workspace_folder, params.config.as_deref()).await {
            Ok(StatusOutcome::Found(info)) => {
                serde_json::to_string(&info).unwrap_or_else(|e| format!("Error: {e}"))
            }
            Ok(StatusOutcome::NotFound) => r#"{"state":"NotFound"}"#.to_string(),
            Ok(StatusOutcome::Ambiguous(candidates)) => {
                // Surface every candidate's identity, service, and
                // config-file label so the agent has enough info to
                // pick the right `config` and retry without further
                // calls.
                let entries: Vec<_> = candidates
                    .iter()
                    .map(|c| {
                        json!({
                            "id": c.id,
                            "name": c.name,
                            "image": c.image,
                            "state": c.state,
                            "composeService": c.compose_service(),
                            "configFile": c.devcontainer_config_file(),
                        })
                    })
                    .collect();
                json!({
                    "state": "Ambiguous",
                    "candidates": entries,
                    "hint": "Multiple containers match this workspace. Call devcontainer_list_configs and re-run with the `config` parameter set to the desired devcontainer.json.",
                })
                .to_string()
            }
            Err(e) => format!("Error: {e}"),
        }
    }
}
