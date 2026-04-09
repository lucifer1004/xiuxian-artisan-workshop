use super::{first_non_empty_value, normalize_key_prefix, open_client, open_optional_client};

#[test]
fn first_non_empty_value_skips_blank_candidates() {
    assert_eq!(
        first_non_empty_value([
            Some("   ".to_string()),
            None,
            Some(" redis://127.0.0.1/ ".to_string()),
        ]),
        Some("redis://127.0.0.1/".to_string())
    );
}

#[test]
fn open_optional_client_returns_none_for_missing_url() {
    assert!(open_optional_client(None).is_none());
}

#[test]
fn open_optional_client_opens_valid_url() {
    let client = open_optional_client(Some("redis://127.0.0.1/".to_string()));
    assert!(client.is_some());
}

#[test]
fn open_client_trims_valid_url() {
    let client = open_client(" redis://127.0.0.1/ ");
    assert!(client.is_ok());
}

#[test]
fn normalize_key_prefix_falls_back_for_blank_input() {
    assert_eq!(
        normalize_key_prefix("   ", "xiuxian:test"),
        "xiuxian:test".to_string()
    );
}

#[test]
fn normalize_key_prefix_trims_non_blank_input() {
    assert_eq!(
        normalize_key_prefix("  xiuxian:custom  ", "xiuxian:test"),
        "xiuxian:custom".to_string()
    );
}
