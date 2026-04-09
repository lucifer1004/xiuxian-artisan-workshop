use std::path::Path;

fn sanitize_path(text: &str, skill_path: &Path) -> String {
    text.replace(skill_path.to_string_lossy().as_ref(), "<SKILL_PATH>")
}

pub fn sanitize_json_paths(value: &mut serde_json::Value, target: &Path) {
    match value {
        serde_json::Value::String(text) => {
            *text = sanitize_path(text, target);
        }
        serde_json::Value::Array(items) => {
            for item in items {
                sanitize_json_paths(item, target);
            }
        }
        serde_json::Value::Object(map) => {
            for (_, child) in map.iter_mut() {
                sanitize_json_paths(child, target);
            }
        }
        _ => {}
    }
}

pub fn canonicalize_json(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut entries: Vec<(String, serde_json::Value)> = map.into_iter().collect();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            let mut sorted = serde_json::Map::new();
            for (key, child) in entries {
                sorted.insert(key, canonicalize_json(child));
            }
            serde_json::Value::Object(sorted)
        }
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(canonicalize_json).collect())
        }
        scalar => scalar,
    }
}
