use crate::tools::common::format_output;
use crate::tools::DevContainerMcp;
use devcontainer_mcp_core::devpod;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodProviderAddParams {
    #[schemars(description = "Provider name or URL to add")]
    provider: String,
    #[serde(default)]
    #[schemars(description = "Additional options as space-separated KEY=VALUE pairs")]
    options: Option<String>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct DevpodProviderDeleteParams {
    #[schemars(description = "Provider name to delete")]
    provider: String,
}

#[tool_router(router = devpod_provider_router, vis = "pub(super)")]
impl DevContainerMcp {
    #[tool(
        name = "devpod_provider_list",
        description = "List all configured DevPod providers."
    )]
    async fn devpod_provider_list(&self) -> String {
        match devpod::provider_list().await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(name = "devpod_provider_add", description = "Add a DevPod provider.")]
    async fn devpod_provider_add(
        &self,
        Parameters(params): Parameters<DevpodProviderAddParams>,
    ) -> String {
        let opt_parts: Vec<String> = params
            .options
            .as_deref()
            .and_then(shlex::split)
            .unwrap_or_default();
        let opt_refs: Vec<&str> = opt_parts.iter().map(|s| s.as_str()).collect();
        match devpod::provider_add(&params.provider, &opt_refs).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "devpod_provider_delete",
        description = "Delete a DevPod provider."
    )]
    async fn devpod_provider_delete(
        &self,
        Parameters(params): Parameters<DevpodProviderDeleteParams>,
    ) -> String {
        match devpod::provider_delete(&params.provider).await {
            Ok(output) => format_output(&output),
            Err(e) => format!("Error: {e}"),
        }
    }
}
