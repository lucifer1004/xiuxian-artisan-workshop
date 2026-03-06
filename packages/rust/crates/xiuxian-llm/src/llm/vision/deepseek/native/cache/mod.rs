mod fingerprint;
mod key;
mod local;
mod text;
mod valkey;

pub(super) use self::fingerprint::prepared_pixels_fingerprint;
pub(in crate::llm::vision::deepseek::native) use self::fingerprint::{
    fingerprint_cache_clear_for_tests, fingerprint_cache_len_for_tests,
};
pub(super) use self::key::build_cache_key;
pub(in crate::llm::vision::deepseek::native) use self::local::{
    local_clear_for_tests, local_set_with_max_entries_for_tests,
};
pub(super) use self::local::{local_get, local_get_shared, local_set};
pub(in crate::llm::vision::deepseek::native) use self::text::{
    normalize_cache_text_owned_for_tests, normalize_cache_text_view_for_tests,
};
pub(super) use self::text::{normalize_owned_non_empty, trim_non_empty};
pub(in crate::llm::vision::deepseek::native) use self::valkey::{
    normalize_valkey_timeout_ms_for_tests, valkey_get_with_for_tests, valkey_set_with_for_tests,
};
pub(super) use self::valkey::{valkey_get, valkey_set};
