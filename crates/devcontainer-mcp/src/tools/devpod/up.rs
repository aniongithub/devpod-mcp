use devcontainer_mcp_core::devpod;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::Meta;
use rmcp::service::Peer;
use rmcp::{tool, tool_router, RoleServer};
use tokio_util::sync::CancellationToken;

use crate::tools::common::{format_output, progress_sink_from_meta};
use crate::tools::DevContainerMcp;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodUpParams {
    #[schemars(
        description = "All arguments for 'devpod up', e.g. 'https://github.com/org/repo --provider docker --id my-ws'"
    )]
    args: String,
}

#[tool_router(router = devpod_up_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devpod_up",
        description = "Create and start a DevPod workspace. Pass the source (git URL, local path, or image) and any flags as space-separated args. Returns full build output for self-healing."
    )]
    async fn devpod_up(
        &self,
        Parameters(params): Parameters<DevpodUpParams>,
        ct: CancellationToken,
        peer: Peer<RoleServer>,
        meta: Meta,
    ) -> String {
        let parts: Vec<String> = shlex::split(&params.args)
            .unwrap_or_else(|| params.args.split_whitespace().map(String::from).collect());
        let part_refs: Vec<&str> = parts.iter().map(|s| s.as_str()).collect();
        let sink = progress_sink_from_meta(&meta, &peer);
        match devpod::up_streaming(&part_refs, &ct, sink).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
