pub(crate) fn normalize_command_input(input: &str) -> &str {
    let mut normalized = input.trim();
    if normalized.starts_with('[')
        && let Some(end) = normalized.find(']')
    {
        let tag = &normalized[1..end];
        if tag.to_ascii_lowercase().starts_with("bbx-") {
            normalized = normalized[end + 1..].trim_start();
        }
    }
    normalized.trim_start_matches('/')
}

pub(crate) fn slice_original_command_suffix<'a>(
    normalized: &'a str,
    lowered_suffix: &str,
) -> Option<&'a str> {
    let start = normalized.len().checked_sub(lowered_suffix.len())?;
    normalized
        .get(start..)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}
