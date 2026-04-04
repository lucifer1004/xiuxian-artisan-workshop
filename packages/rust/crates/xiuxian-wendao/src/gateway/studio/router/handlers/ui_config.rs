//! UI configuration endpoint handlers for Studio API.

use std::sync::Arc;

use axum::{Json, extract::State};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::UiConfig;

/// Gets the current UI configuration.
///
/// # Errors
///
/// This handler currently does not produce handler-local errors.
pub async fn get(State(state): State<Arc<GatewayState>>) -> Result<Json<UiConfig>, StudioApiError> {
    Ok(Json(state.studio.ui_config()))
}

/// Sets the live UI configuration for the current gateway process.
pub async fn set(
    State(state): State<Arc<GatewayState>>,
    Json(config_value): Json<UiConfig>,
) -> Json<UiConfig> {
    state.studio.set_ui_config(config_value);
    Json(state.studio.ui_config())
}
