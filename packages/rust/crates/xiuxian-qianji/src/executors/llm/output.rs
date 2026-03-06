use serde_json::{Value, json};

fn parse_json_from_text(raw: &str) -> Option<Value> {
    let text = raw.trim();
    if text.is_empty() {
        return None;
    }

    let strip_fence = |candidate: &str| -> String {
        let without_open = candidate
            .strip_prefix("```json")
            .or_else(|| candidate.strip_prefix("```JSON"))
            .or_else(|| candidate.strip_prefix("```"))
            .unwrap_or(candidate)
            .trim()
            .to_string();
        without_open
            .strip_suffix("```")
            .unwrap_or(&without_open)
            .trim()
            .to_string()
    };

    let mut candidates = vec![strip_fence(text)];
    let fence_stripped = candidates[0].clone();

    let list_start = fence_stripped.find('[');
    let list_end = fence_stripped.rfind(']');
    if let (Some(start), Some(end)) = (list_start, list_end)
        && end > start
    {
        candidates.push(fence_stripped[start..=end].to_string());
    }

    let obj_start = fence_stripped.find('{');
    let obj_end = fence_stripped.rfind('}');
    if let (Some(start), Some(end)) = (obj_start, obj_end)
        && end > start
    {
        candidates.push(fence_stripped[start..=end].to_string());
    }

    for candidate in candidates {
        if let Ok(value) = serde_json::from_str::<Value>(&candidate) {
            return Some(value);
        }
    }
    None
}

fn build_repo_tree_fallback_plan(context: &Value) -> Value {
    let repo_tree = context
        .get("repo_tree")
        .and_then(Value::as_str)
        .unwrap_or("");
    let mut paths = Vec::new();
    for line in repo_tree.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("./") {
            continue;
        }
        if trimmed.matches('/').count() > 1 {
            continue;
        }
        let path = trimmed.trim_start_matches("./").trim();
        if !path.is_empty() {
            paths.push(path.to_string());
        }
        if paths.len() >= 12 {
            break;
        }
    }
    if paths.is_empty() {
        paths.push(".".to_string());
    }
    json!([
        {
            "shard_id": "repository-overview",
            "paths": paths,
        }
    ])
}

pub(super) fn build_output_data(
    output_key: &str,
    parse_json_output: bool,
    fallback_repo_tree_on_parse_failure: bool,
    context: &Value,
    conclusion: String,
) -> serde_json::Map<String, Value> {
    let mut data = serde_json::Map::new();
    if parse_json_output {
        let parsed = parse_json_from_text(&conclusion).or_else(|| {
            if fallback_repo_tree_on_parse_failure {
                Some(build_repo_tree_fallback_plan(context))
            } else {
                None
            }
        });
        data.insert(
            output_key.to_string(),
            parsed.unwrap_or_else(|| Value::Array(Vec::new())),
        );
        data.insert(format!("{output_key}_raw"), Value::String(conclusion));
    } else {
        data.insert(output_key.to_string(), Value::String(conclusion));
    }
    data
}
