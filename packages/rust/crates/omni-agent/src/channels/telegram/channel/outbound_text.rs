use serde_json::Value;

const EXTRACTION_MAX_DEPTH: usize = 6;
const PREFERRED_TEXT_KEYS: &[&str] = &[
    "content", "message", "result", "data", "output", "error", "details",
];

pub(super) fn normalize_telegram_outbound_text(message: &str) -> String {
    extract_display_text_from_json_envelope(message).unwrap_or_else(|| message.to_string())
}

fn extract_display_text_from_json_envelope(message: &str) -> Option<String> {
    let trimmed = message.trim();
    if !(trimmed.starts_with('{') || trimmed.starts_with('[')) {
        return None;
    }

    let value: Value = serde_json::from_str(trimmed).ok()?;
    let extracted = extract_text_from_value(&value, EXTRACTION_MAX_DEPTH)?;
    (!extracted.trim().is_empty()).then_some(extracted)
}

fn extract_text_from_value(value: &Value, depth: usize) -> Option<String> {
    if depth == 0 {
        return None;
    }

    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => extract_text_from_array(items, depth - 1),
        Value::Object(map) => {
            if map
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind.eq_ignore_ascii_case("text"))
                && let Some(text) = map.get("text").and_then(Value::as_str)
            {
                return Some(text.to_string());
            }

            for key in PREFERRED_TEXT_KEYS {
                if let Some(candidate) = map.get(*key)
                    && let Some(text) = extract_text_from_value(candidate, depth - 1)
                    && !text.trim().is_empty()
                {
                    return Some(text);
                }
            }

            map.get("text")
                .and_then(Value::as_str)
                .map(std::string::ToString::to_string)
        }
        _ => None,
    }
}

fn extract_text_from_array(items: &[Value], depth: usize) -> Option<String> {
    let parts: Vec<String> = items
        .iter()
        .filter_map(|item| extract_text_from_value(item, depth))
        .filter(|text| !text.trim().is_empty())
        .collect();
    (!parts.is_empty()).then(|| parts.join("\n"))
}
