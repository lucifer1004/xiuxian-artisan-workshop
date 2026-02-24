const DEFAULT_MINIMAX_API_BASE: &str = "https://api.minimax.io/v1";

pub(in crate::llm) fn normalize_minimax_api_base(base_url: Option<&str>) -> String {
    base_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map_or_else(|| DEFAULT_MINIMAX_API_BASE.to_string(), ToString::to_string)
}

pub(in crate::llm) fn normalize_minimax_model(raw: &str) -> String {
    let mut model = raw.trim().to_string();
    if let Some(stripped) = model.strip_prefix("minimax/") {
        model = stripped.to_string();
    }
    if let Some(stripped) = model.strip_prefix("minimax:") {
        model = stripped.to_string();
    }

    let lower = model.to_ascii_lowercase();
    if let Some(suffix_lower) = lower.strip_prefix("minimax-") {
        let mut normalized = format!("MiniMax-{}", &model[8..]);
        if suffix_lower.ends_with("-highspeed") {
            let trim_len = normalized.len().saturating_sub("-highspeed".len());
            normalized.truncate(trim_len);
            normalized.push_str("-lightning");
        }
        return normalized;
    }
    model
}
