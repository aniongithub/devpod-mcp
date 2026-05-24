use devcontainer_mcp_core::devcontainer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::Meta;
use rmcp::service::Peer;
use rmcp::{tool, tool_router, RoleServer};
use tokio_util::sync::CancellationToken;

use crate::tools::common::{format_output, progress_sink_from_meta};
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerUpParams {
    #[schemars(
        description = "Path to the workspace folder containing .devcontainer/devcontainer.json"
    )]
    workspace_folder: String,
    #[schemars(description = "Path to a specific devcontainer.json (overrides auto-detection)")]
    config: Option<String>,
    #[schemars(
        description = "Additional flags as space-separated args, e.g. '--remove-existing-container --build-no-cache'"
    )]
    extra_args: Option<String>,
}

#[tool_router(router = devcontainer_up_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devcontainer_up",
        description = "Create and start a local dev container using the devcontainer CLI. Requires a workspace folder with a devcontainer.json."
    )]
    async fn devcontainer_up(
        &self,
        Parameters(params): Parameters<DevcontainerUpParams>,
        ct: CancellationToken,
        peer: Peer<RoleServer>,
        meta: Meta,
    ) -> String {
        let extra: Vec<String> = params
            .extra_args
            .as_deref()
            .and_then(shlex::split)
            .unwrap_or_default();
        let extra_refs: Vec<&str> = extra.iter().map(|s| s.as_str()).collect();
        let sink = progress_sink_from_meta(&meta, &peer);
        match devcontainer::up_streaming(
            &params.workspace_folder,
            params.config.as_deref(),
            &extra_refs,
            &ct,
            sink,
        )
        .await
        {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
