use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::config::UiPluginArtifact;
use crate::link_graph::runtime_config::{
    render_link_graph_plugin_artifact_toml_for_selector,
    resolve_link_graph_plugin_artifact_for_selector,
};
use crate::zhenfa_router::native::WendaoPluginArtifactOutputFormat;
use xiuxian_wendao_core::artifacts::PluginArtifactSelector;

use crate::gateway::studio::router::handlers::capabilities::types::{
    PluginArtifactPath, PluginArtifactQuery,
};

fn render_plugin_artifact_json_response(
    selector: &PluginArtifactSelector,
) -> Result<Response, StudioApiError> {
    let artifact = resolve_link_graph_plugin_artifact_for_selector(selector).ok_or_else(|| {
        StudioApiError::internal(
            "PLUGIN_ARTIFACT_RESOLVE_FAILED",
            "Failed to resolve plugin artifact",
            None,
        )
    })?;

    Ok(Json(UiPluginArtifact::from(artifact)).into_response())
}

fn render_plugin_artifact_toml_response(
    selector: &PluginArtifactSelector,
) -> Result<Response, StudioApiError> {
    let body = render_link_graph_plugin_artifact_toml_for_selector(selector)
        .map_err(|error| {
            StudioApiError::internal(
                "PLUGIN_ARTIFACT_EXPORT_FAILED",
                "Failed to render plugin artifact as TOML",
                Some(error.to_string()),
            )
        })?
        .ok_or_else(|| {
            StudioApiError::internal(
                "PLUGIN_ARTIFACT_EXPORT_FAILED",
                "Failed to render plugin artifact as TOML",
                None,
            )
        })?;

    Ok((
        StatusCode::OK,
        [(
            CONTENT_TYPE,
            HeaderValue::from_static("text/plain; charset=utf-8"),
        )],
        body,
    )
        .into_response())
}

/// Read the currently resolved generic plugin artifact used by runtime config.
///
/// # Errors
///
/// This handler currently does not produce handler-local errors.
pub async fn get_plugin_artifact(
    State(_state): State<Arc<GatewayState>>,
    Path(path): Path<PluginArtifactPath>,
    Query(query): Query<PluginArtifactQuery>,
) -> Result<Response, StudioApiError> {
    let selector = PluginArtifactSelector::from(path);

    match query
        .format
        .unwrap_or(WendaoPluginArtifactOutputFormat::Json)
    {
        WendaoPluginArtifactOutputFormat::Json => render_plugin_artifact_json_response(&selector),
        WendaoPluginArtifactOutputFormat::Toml => render_plugin_artifact_toml_response(&selector),
    }
}

#[cfg(all(test, feature = "julia"))]
mod tests {
    use crate::gateway::studio::router::handlers::capabilities::types::{
        PluginArtifactPath, PluginArtifactQuery,
    };
    use crate::gateway::studio::router::{GatewayState, StudioState};
    use crate::gateway::studio::types::config::UiPluginArtifact;
    use crate::set_link_graph_wendao_config_override;
    use crate::zhenfa_router::native::WendaoPluginArtifactOutputFormat;
    use axum::body::to_bytes;
    use axum::extract::{Path, Query, State};
    use serial_test::serial;
    use std::fs;
    use std::sync::Arc;
    use xiuxian_wendao_builtin::{
        linked_builtin_julia_gateway_artifact_path,
        linked_builtin_julia_gateway_artifact_runtime_config_toml,
        linked_builtin_julia_gateway_artifact_schema_version,
    };

    #[tokio::test]
    #[serial]
    async fn generic_plugin_artifact_handler_returns_plugin_artifact() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let config_path = temp.path().join("wendao.toml");
        let (plugin_id, artifact_id) = linked_builtin_julia_gateway_artifact_path();
        fs::write(
            &config_path,
            linked_builtin_julia_gateway_artifact_runtime_config_toml(None),
        )
        .unwrap_or_else(|error| panic!("write config: {error}"));
        let config_path_string = config_path.to_string_lossy().to_string();
        set_link_graph_wendao_config_override(&config_path_string);

        let state = Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            webhook_url: None,
            studio: Arc::new(StudioState::new()),
        });

        let response = super::get_plugin_artifact(
            State(Arc::clone(&state)),
            Path(PluginArtifactPath {
                plugin_id: plugin_id.clone(),
                artifact_id: artifact_id.clone(),
            }),
            Query(PluginArtifactQuery {
                format: Some(WendaoPluginArtifactOutputFormat::Json),
            }),
        )
        .await
        .unwrap_or_else(|error| {
            panic!("generic deployment artifact handler should resolve: {error:?}")
        });

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("read json body: {error}"));
        let artifact: UiPluginArtifact = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("decode artifact json: {error}"));

        assert_eq!(artifact.plugin_id, plugin_id);
        assert_eq!(artifact.artifact_id, artifact_id);
        assert_eq!(
            artifact.schema_version.as_deref(),
            Some(linked_builtin_julia_gateway_artifact_schema_version())
        );
        assert_eq!(artifact.route.as_deref(), Some("/rerank"));
    }
}
