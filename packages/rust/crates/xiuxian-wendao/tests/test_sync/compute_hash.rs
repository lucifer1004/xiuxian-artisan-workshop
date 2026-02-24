#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::implicit_clone,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::manual_string_new,
    clippy::needless_raw_string_hashes,
    clippy::format_push_string,
    clippy::map_unwrap_or,
    clippy::unnecessary_to_owned,
    clippy::too_many_lines
)]
#[test]
fn test_compute_hash() {
    use xiuxian_wendao::SyncEngine;

    let hash1 = SyncEngine::compute_hash("hello world");
    let hash2 = SyncEngine::compute_hash("hello world");
    let hash3 = SyncEngine::compute_hash("different");

    assert_eq!(hash1, hash2);
    assert_ne!(hash1, hash3);
    // xxhash produces 16 character hex
    assert_eq!(hash1.len(), 16);
}
