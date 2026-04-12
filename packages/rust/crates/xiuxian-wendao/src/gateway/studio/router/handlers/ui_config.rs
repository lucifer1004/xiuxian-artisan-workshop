//! UI configuration endpoint handlers for Studio API.

use std::sync::Arc;

use axum::{Json, extract::State};

use crate::gateway::studio::router::{
    GatewayState, StudioApiError, persist_ui_config_to_wendao_toml,
    persist_ui_config_to_wendao_toml_path, studio_wendao_toml_path,
};
use crate::gateway::studio::types::UiConfig;

/// Gets the current UI configuration.
///
/// # Errors
///
/// This handler currently does not produce handler-local errors.
pub async fn get(State(state): State<Arc<GatewayState>>) -> Result<Json<UiConfig>, StudioApiError> {
    Ok(Json(state.studio.ui_config()))
}

/// Sets and persists the live UI configuration for the current gateway process.
///
/// # Errors
///
/// Returns [`StudioApiError`] when the Wendao TOML cannot be persisted.
pub async fn set(
    State(state): State<Arc<GatewayState>>,
    Json(config_value): Json<UiConfig>,
) -> Result<Json<UiConfig>, StudioApiError> {
    let persist_result = if let Some(config_path) = state.studio.bootstrap_config_path.as_deref() {
        persist_ui_config_to_wendao_toml_path(config_path, &config_value)
    } else {
        persist_ui_config_to_wendao_toml(state.studio.config_root.as_path(), &config_value)
    };
    persist_result.map_err(|details| {
        let config_path = studio_wendao_toml_path(state.studio.config_root.as_path());
        StudioApiError::internal(
            "UI_CONFIG_PERSIST_FAILED",
            format!(
                "failed to persist Studio UI config to `{}`",
                config_path.display()
            ),
            Some(details),
        )
    })?;
    state.studio.set_ui_config(config_value);
    Ok(Json(state.studio.ui_config()))
}
