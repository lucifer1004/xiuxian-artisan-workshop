pub(crate) fn live_valkey_enabled() -> bool {
    std::env::var("XIUXIAN_DAOCHANG_LIVE_VALKEY")
        .ok()
        .map(|raw| raw.trim().to_ascii_lowercase())
        .is_some_and(|raw| matches!(raw.as_str(), "1" | "true" | "yes" | "on"))
}

pub(crate) fn resolve_live_valkey_url() -> Option<String> {
    for key in ["VALKEY_URL", "XIUXIAN_WENDAO_VALKEY_URL"] {
        if let Ok(url) = std::env::var(key)
            && !url.trim().is_empty()
        {
            return Some(url);
        }
    }
    None
}

pub(crate) fn resolve_enabled_live_valkey_url(skip_context: &str) -> Option<String> {
    if !live_valkey_enabled() {
        return None;
    }
    let Some(url) = resolve_live_valkey_url() else {
        eprintln!("skip: set VALKEY_URL or XIUXIAN_WENDAO_VALKEY_URL for {skip_context}");
        return None;
    };
    Some(url)
}
