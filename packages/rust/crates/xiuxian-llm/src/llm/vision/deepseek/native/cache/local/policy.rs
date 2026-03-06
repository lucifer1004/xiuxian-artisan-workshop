use super::super::super::env::parse_env_usize;

const CACHE_MAX_ENTRIES_ENV: &str = "XIUXIAN_VISION_OCR_CACHE_LOCAL_MAX_ENTRIES";
const DEFAULT_MAX_ENTRIES: usize = 1_024;

pub(super) fn resolve_max_entries() -> usize {
    normalize_max_entries(parse_env_usize(CACHE_MAX_ENTRIES_ENV).unwrap_or(DEFAULT_MAX_ENTRIES))
}

pub(super) fn normalize_max_entries(max_entries: usize) -> usize {
    max_entries.max(1)
}

pub(super) fn should_clear_before_insert(current_len: usize, max_entries: usize) -> bool {
    current_len >= max_entries
}
