use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderValue, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::UiJuliaDeploymentArtifact;
use crate::resolve_link_graph_julia_deployment_artifact;
use crate::zhenfa_router::native::{
    WendaoJuliaDeploymentArtifactOutputFormat, render_julia_deployment_artifact_toml,
};

use crate::gateway::studio::router::handlers::capabilities::types::JuliaDeploymentArtifactQuery;

/// Read the currently resolved Julia deployment artifact used by runtime config.
///
/// # Errors
///
/// This handler currently does not produce handler-local errors.
pub async fn get_julia_deployment_artifact(
    State(_state): State<Arc<GatewayState>>,
    Query(query): Query<JuliaDeploymentArtifactQuery>,
) -> Result<Response, StudioApiError> {
    match query
        .format
        .unwrap_or(WendaoJuliaDeploymentArtifactOutputFormat::Json)
    {
        WendaoJuliaDeploymentArtifactOutputFormat::Json => Ok(Json(
            UiJuliaDeploymentArtifact::from(resolve_link_graph_julia_deployment_artifact()),
        )
        .into_response()),
        WendaoJuliaDeploymentArtifactOutputFormat::Toml => {
            let body = render_julia_deployment_artifact_toml().map_err(|error| {
                StudioApiError::internal(
                    "JULIA_DEPLOYMENT_ARTIFACT_EXPORT_FAILED",
                    "Failed to render Julia deployment artifact as TOML",
                    Some(error.to_string()),
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
    }
}
