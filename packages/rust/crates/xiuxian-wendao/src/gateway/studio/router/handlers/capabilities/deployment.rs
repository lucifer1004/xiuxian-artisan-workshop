use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderValue, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::config::UiPluginArtifact;
use crate::gateway::studio::types::config::UiPluginLaunchSpec;
use crate::link_graph::julia_deployment_artifact_selector;
use crate::link_graph::plugin_runtime::{
    render_plugin_artifact_toml_for_selector, resolve_plugin_artifact_for_selector,
};
use crate::zhenfa_router::native::WendaoCompatDeploymentArtifactOutputFormat;
use xiuxian_wendao_core::artifacts::PluginArtifactSelector;

use crate::gateway::studio::router::handlers::capabilities::types::{
    CompatDeploymentArtifactQuery, PluginArtifactPath, PluginArtifactQuery,
};

fn build_compat_deployment_artifact_json(
    value: UiPluginArtifact,
) -> Result<Value, StudioApiError> {
    let mut json = serde_json::to_value(value).map_err(|error| {
        StudioApiError::internal(
            "COMPAT_DEPLOYMENT_ARTIFACT_SERIALIZE_FAILED",
            "Failed to serialize compatibility deployment artifact",
            Some(error.to_string()),
        )
    })?;

    let Some(object) = json.as_object_mut() else {
        return Err(StudioApiError::internal(
            "COMPAT_DEPLOYMENT_ARTIFACT_SERIALIZE_FAILED",
            "Failed to serialize compatibility deployment artifact",
            Some("serialized payload was not a JSON object".to_string()),
        ));
    };

    object.remove("pluginId");
    object.remove("artifactId");

    let launch = object.remove("launch").unwrap_or(Value::Null);
    object.insert(
        "launch".to_string(),
        match launch {
            Value::Object(_) => launch,
            _ => json!({
                "launcherPath": UiPluginLaunchSpec::default().launcher_path,
                "args": UiPluginLaunchSpec::default().args,
            }),
        },
    );

    Ok(json)
}

fn render_plugin_artifact_json_response(
    selector: &PluginArtifactSelector,
) -> Result<Response, StudioApiError> {
    let artifact = resolve_plugin_artifact_for_selector(selector).ok_or_else(|| {
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
    let body = render_plugin_artifact_toml_for_selector(selector)
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

fn render_compat_deployment_artifact_json_response() -> Result<Response, StudioApiError> {
    let artifact = resolve_plugin_artifact_for_selector(&julia_deployment_artifact_selector())
        .ok_or_else(|| {
            StudioApiError::internal(
                "COMPAT_DEPLOYMENT_ARTIFACT_RESOLVE_FAILED",
                "Failed to resolve compatibility deployment artifact",
                None,
            )
        })?;

    Ok(Json(build_compat_deployment_artifact_json(
        UiPluginArtifact::from(artifact),
    )?)
    .into_response())
}

fn render_compat_deployment_artifact_toml_response() -> Result<Response, StudioApiError> {
    let body = render_plugin_artifact_toml_for_selector(&julia_deployment_artifact_selector())
        .map_err(|error| {
            StudioApiError::internal(
                "COMPAT_DEPLOYMENT_ARTIFACT_EXPORT_FAILED",
                "Failed to render compatibility deployment artifact as TOML",
                Some(error.to_string()),
            )
        })?
        .ok_or_else(|| {
            StudioApiError::internal(
                "COMPAT_DEPLOYMENT_ARTIFACT_EXPORT_FAILED",
                "Failed to render compatibility deployment artifact as TOML",
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
        .unwrap_or(WendaoCompatDeploymentArtifactOutputFormat::Json)
    {
        WendaoCompatDeploymentArtifactOutputFormat::Json => {
            render_plugin_artifact_json_response(&selector)
        }
        WendaoCompatDeploymentArtifactOutputFormat::Toml => {
            render_plugin_artifact_toml_response(&selector)
        }
    }
}

/// Read the currently resolved compatibility deployment artifact used by runtime config.
///
/// # Errors
///
/// This handler currently does not produce handler-local errors.
pub async fn get_compat_deployment_artifact(
    State(_state): State<Arc<GatewayState>>,
    Query(query): Query<CompatDeploymentArtifactQuery>,
) -> Result<Response, StudioApiError> {
    match query
        .format
        .unwrap_or(WendaoCompatDeploymentArtifactOutputFormat::Json)
    {
        WendaoCompatDeploymentArtifactOutputFormat::Json => {
            render_compat_deployment_artifact_json_response()
        }
        WendaoCompatDeploymentArtifactOutputFormat::Toml => {
            render_compat_deployment_artifact_toml_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::build_compat_deployment_artifact_json;
    use crate::gateway::studio::router::handlers::capabilities::types::{
        CompatDeploymentArtifactQuery, PluginArtifactPath, PluginArtifactQuery,
    };
    use crate::gateway::studio::router::{GatewayState, StudioState};
    use crate::gateway::studio::types::config::{UiPluginArtifact, UiPluginLaunchSpec};
    use crate::set_link_graph_wendao_config_override;
    use crate::zhenfa_router::native::WendaoCompatDeploymentArtifactOutputFormat;
    use axum::body::to_bytes;
    use axum::extract::{Path, Query, State};
    use serde_json::json;
    use serial_test::serial;
    use std::fs;
    use std::sync::Arc;
    use xiuxian_wendao_julia::compatibility::link_graph::DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH;

    #[test]
    fn compat_ui_artifact_json_builds_from_generic_ui_artifact() {
        let artifact = UiPluginArtifact {
            plugin_id: "xiuxian-wendao-julia".to_string(),
            artifact_id: "deployment".to_string(),
            artifact_schema_version: "v1".to_string(),
            generated_at: "2026-03-27T12:00:00Z".to_string(),
            base_url: Some("http://127.0.0.1:8088".to_string()),
            route: Some("/arrow-ipc".to_string()),
            health_route: Some("/healthz".to_string()),
            timeout_secs: Some(15),
            schema_version: Some("v1".to_string()),
            launch: Some(UiPluginLaunchSpec {
                launcher_path: DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH.to_string(),
                args: vec!["--service-mode".to_string(), "stream".to_string()],
            }),
        };

        let json = build_compat_deployment_artifact_json(artifact)
            .unwrap_or_else(|error| panic!("build compat ui artifact json: {error:?}"));

        assert_eq!(json["artifactSchemaVersion"], "v1");
        assert_eq!(json["generatedAt"], "2026-03-27T12:00:00Z");
        assert_eq!(json["baseUrl"], "http://127.0.0.1:8088");
        assert_eq!(json["route"], "/arrow-ipc");
        assert_eq!(json["schemaVersion"], "v1");
        assert_eq!(json["timeoutSecs"], 15);
        assert_eq!(
            json["launch"]["launcherPath"],
            DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH
        );
        assert_eq!(
            json["launch"]["args"],
            serde_json::json!(["--service-mode", "stream"])
        );
        assert!(json.get("pluginId").is_none());
        assert!(json.get("artifactId").is_none());
    }

    #[test]
    fn compat_ui_artifact_serializes_legacy_json_shape() {
        let artifact = UiPluginArtifact {
            plugin_id: "xiuxian-wendao-julia".to_string(),
            artifact_id: "deployment".to_string(),
            artifact_schema_version: "v1".to_string(),
            generated_at: "2026-03-27T12:00:00Z".to_string(),
            base_url: Some("http://127.0.0.1:8088".to_string()),
            route: Some("/arrow-ipc".to_string()),
            health_route: Some("/healthz".to_string()),
            timeout_secs: Some(15),
            schema_version: Some("v1".to_string()),
            launch: Some(UiPluginLaunchSpec {
                launcher_path: DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH.to_string(),
                args: vec!["--service-mode".to_string(), "stream".to_string()],
            }),
        };

        let json = build_compat_deployment_artifact_json(artifact)
            .unwrap_or_else(|error| panic!("serialize legacy compat ui artifact: {error:?}"));

        assert_eq!(json["artifactSchemaVersion"], "v1");
        assert_eq!(json["generatedAt"], "2026-03-27T12:00:00Z");
        assert_eq!(json["baseUrl"], "http://127.0.0.1:8088");
        assert_eq!(json["route"], "/arrow-ipc");
        assert_eq!(json["schemaVersion"], "v1");
        assert_eq!(
            json["launch"]["launcherPath"],
            DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH
        );
        assert_eq!(
            json["launch"]["args"],
            serde_json::json!(["--service-mode", "stream"])
        );
    }

    #[test]
    fn compat_ui_artifact_json_fills_default_launch_shape_when_missing() {
        let artifact = UiPluginArtifact {
            plugin_id: "xiuxian-wendao-julia".to_string(),
            artifact_id: "deployment".to_string(),
            artifact_schema_version: "v1".to_string(),
            generated_at: "2026-03-27T12:00:00Z".to_string(),
            base_url: None,
            route: None,
            health_route: None,
            timeout_secs: None,
            schema_version: None,
            launch: None,
        };

        let json = build_compat_deployment_artifact_json(artifact)
            .unwrap_or_else(|error| panic!("build compat ui artifact json with default launch: {error:?}"));

        assert_eq!(json["launch"], json!({"launcherPath": "", "args": []}));
        assert!(json.get("pluginId").is_none());
        assert!(json.get("artifactId").is_none());
    }

    #[tokio::test]
    #[serial]
    async fn legacy_julia_deployment_artifact_handler_alias_still_works() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let config_path = temp.path().join("wendao.toml");
        fs::write(
            &config_path,
            r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:18080"
route = "/arrow-ipc"
schema_version = "v1"
service_mode = "stream"
"#,
        )
        .unwrap_or_else(|error| panic!("write config: {error}"));
        let config_path_string = config_path.to_string_lossy().to_string();
        set_link_graph_wendao_config_override(&config_path_string);

        let state = Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(StudioState::new()),
        });

        let response = super::get_compat_deployment_artifact(
            State(Arc::clone(&state)),
            Query(CompatDeploymentArtifactQuery {
                format: Some(WendaoCompatDeploymentArtifactOutputFormat::Json),
            }),
        )
        .await
        .unwrap_or_else(|error| {
            panic!("legacy deployment artifact handler should resolve: {error:?}")
        });

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap_or_else(|error| panic!("read json body: {error}"));
        let artifact: serde_json::Value = serde_json::from_slice(&body)
            .unwrap_or_else(|error| panic!("decode artifact json: {error}"));

        assert_eq!(artifact["schemaVersion"], "v1");
        assert_eq!(artifact["route"], "/arrow-ipc");
        assert!(artifact["launch"].is_object());
    }

    #[tokio::test]
    #[serial]
    async fn generic_plugin_artifact_handler_returns_plugin_artifact() {
        let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let config_path = temp.path().join("wendao.toml");
        fs::write(
            &config_path,
            r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:18080"
route = "/arrow-ipc"
schema_version = "v1"
service_mode = "stream"
"#,
        )
        .unwrap_or_else(|error| panic!("write config: {error}"));
        let config_path_string = config_path.to_string_lossy().to_string();
        set_link_graph_wendao_config_override(&config_path_string);

        let state = Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(StudioState::new()),
        });

        let response = super::get_plugin_artifact(
            State(Arc::clone(&state)),
            Path(PluginArtifactPath {
                plugin_id: "xiuxian-wendao-julia".to_string(),
                artifact_id: "deployment".to_string(),
            }),
            Query(PluginArtifactQuery {
                format: Some(WendaoCompatDeploymentArtifactOutputFormat::Json),
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

        assert_eq!(artifact.plugin_id, "xiuxian-wendao-julia");
        assert_eq!(artifact.artifact_id, "deployment");
        assert_eq!(artifact.schema_version.as_deref(), Some("v1"));
        assert_eq!(artifact.route.as_deref(), Some("/arrow-ipc"));
    }
}
