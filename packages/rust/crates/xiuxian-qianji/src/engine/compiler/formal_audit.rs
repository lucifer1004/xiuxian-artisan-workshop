use crate::contracts::NodeDefinition;
use crate::error::QianjiError;

pub(super) fn retry_targets(node_def: &NodeDefinition) -> Vec<String> {
    node_def
        .params
        .get("retry_targets")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn uses_llm_controller(node_def: &NodeDefinition) -> bool {
    node_def.qianhuan.is_some() && node_def.llm.is_some()
}

pub(super) fn ensure_native_retry_budget_not_configured(
    node_def: &NodeDefinition,
) -> Result<(), QianjiError> {
    if node_def.params.get("max_retries").is_some() {
        return Err(QianjiError::Topology(
            "formal_audit.max_retries requires `[nodes.qianhuan] + [nodes.llm]`; native formal_audit only supports retry_targets.".to_string(),
        ));
    }
    Ok(())
}

#[cfg(feature = "llm")]
pub(super) fn threshold_score(node_def: &NodeDefinition) -> Result<f32, QianjiError> {
    let raw = node_def
        .params
        .get("threshold_score")
        .map_or(Ok(0.8_f32), |value| {
            serde_json::from_value::<f32>(value.clone()).map_err(|_error| {
                QianjiError::Topology(
                    "formal_audit.threshold_score must be a finite number".to_string(),
                )
            })
        })?;
    if !raw.is_finite() {
        return Err(QianjiError::Topology(
            "formal_audit.threshold_score must be a finite number".to_string(),
        ));
    }
    if !(0.0..=1.0).contains(&raw) {
        return Err(QianjiError::Topology(
            "formal_audit.threshold_score must be within [0.0, 1.0]".to_string(),
        ));
    }
    Ok(raw)
}

#[cfg(feature = "llm")]
pub(super) fn output_key(node_def: &NodeDefinition) -> String {
    node_def
        .params
        .get("output_key")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("audit_critique")
        .to_string()
}

#[cfg(feature = "llm")]
pub(super) fn max_retries(node_def: &NodeDefinition) -> Result<u32, QianjiError> {
    let raw = node_def
        .params
        .get("max_retries")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(3);
    u32::try_from(raw).map_err(|_error| {
        QianjiError::Topology("formal_audit.max_retries must fit in u32".to_string())
    })
}

#[cfg(feature = "llm")]
pub(super) fn ensure_llm_retry_targets(node_def: &NodeDefinition) -> Result<(), QianjiError> {
    let max_retries = max_retries(node_def)?;
    if max_retries > 0 && retry_targets(node_def).is_empty() {
        return Err(QianjiError::Topology(
            "formal_audit.retry_targets must be non-empty when LLM-augmented formal_audit.max_retries is greater than 0.".to_string(),
        ));
    }
    Ok(())
}

#[cfg(feature = "llm")]
pub(super) fn retry_counter_key(node_def: &NodeDefinition) -> String {
    node_def
        .params
        .get("retry_counter_key")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("audit_retry_count")
        .to_string()
}

#[cfg(feature = "llm")]
pub(super) fn score_key(node_def: &NodeDefinition) -> String {
    node_def
        .params
        .get("score_key")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("audit_score")
        .to_string()
}

/// Returns whether cognitive supervision is enabled for formal audit.
/// Defaults to `false` for backward compatibility.
#[cfg(feature = "llm")]
pub(super) fn enable_cognitive_supervision(node_def: &NodeDefinition) -> bool {
    node_def
        .params
        .get("enable_cognitive_supervision")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

/// Returns the cognitive early halt threshold for formal audit.
/// Defaults to `0.3` (halt if coherence drops below 30%).
#[cfg(feature = "llm")]
pub(super) fn cognitive_early_halt_threshold(node_def: &NodeDefinition) -> f32 {
    node_def
        .params
        .get("cognitive_early_halt_threshold")
        .map_or(0.3, |value| {
            serde_json::from_value::<f32>(value.clone()).unwrap_or(0.3)
        })
}
