use super::common::{normalize, slice_original};

/// Parse background command forms:
/// - `/bg <prompt>`
/// - `bg <prompt>`
/// - `/research <prompt>`
/// - `research <prompt>` (auto-background because this skill is typically long-running)
pub fn parse_background_prompt(input: &str) -> Option<String> {
    let normalized = normalize(input);
    let lower = normalized.to_ascii_lowercase();

    if let Some(rest) = lower.strip_prefix("bg ") {
        return slice_original(normalized, rest).map(ToString::to_string);
    }
    if let Some(rest) = lower.strip_prefix("research ") {
        let original = slice_original(normalized, rest)?;
        return Some(format!("research {}", original.trim()));
    }
    None
}
