use devcontainer_mcp_core::devcontainer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::Meta;
use rmcp::service::Peer;
use rmcp::{tool, tool_router, RoleServer};
use tokio_util::sync::CancellationToken;

use crate::tools::common::{format_output, progress_sink_from_meta};
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevcontainerBuildParams {
    #[schemars(description = "Path to the workspace folder")]
    workspace_folder: String,
    #[schemars(
        description = "Additional flags as space-separated args, e.g. '--no-cache --image-name my-image'"
    )]
    extra_args: Option<String>,
}

#[tool_router(router = devcontainer_build_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devcontainer_build",
        description = "Build a dev container image without starting it."
    )]
    async fn devcontainer_build(
        &self,
        Parameters(params): Parameters<DevcontainerBuildParams>,
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
        match devcontainer::build_streaming(&params.workspace_folder, &extra_refs, &ct, sink).await
        {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
