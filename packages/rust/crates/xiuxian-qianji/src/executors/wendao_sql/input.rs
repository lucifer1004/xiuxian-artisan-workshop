use serde_json::Value;
pub(super) fn context_string(context: &Value, key: &str) -> Option<String> {
    context.get(key).and_then(value_to_string)
}

pub(super) fn required_context_string(context: &Value, key: &str) -> Result<String, String> {
    context_string(context, key)
        .ok_or_else(|| format!("missing required context string for key `{key}`"))
}

pub(super) fn resolve_project_root(context: &Value, project_root_key: Option<&str>) -> String {
    project_root_key
        .and_then(|key| context_string(context, key))
        .or_else(|| context_string(context, "project_root"))
        .unwrap_or_default()
}

pub(super) fn resolve_endpoint(
    context: &Value,
    static_endpoint: Option<&str>,
    endpoint_key: Option<&str>,
) -> Result<String, String> {
    endpoint_key
        .and_then(|key| context_string(context, key))
        .or_else(|| static_endpoint.and_then(normalize_non_empty))
        .ok_or_else(|| "missing Wendao query endpoint; set endpoint or endpoint_key".to_string())
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(raw) => normalize_non_empty(raw),
        Value::Number(raw) => Some(raw.to_string()),
        Value::Bool(raw) => Some(raw.to_string()),
        _ => None,
    }
}

fn normalize_non_empty(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
