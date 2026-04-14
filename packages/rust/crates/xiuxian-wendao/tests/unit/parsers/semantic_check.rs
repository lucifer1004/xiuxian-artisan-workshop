use crate::parsers::semantic_check::{
    extract_function_args, extract_hash_references, extract_id_references, generate_suggested_id,
    validate_contract,
};

#[test]
fn extract_id_references_only_returns_hash_targets() {
    let refs = extract_id_references("See [[#intro]] and [[regular-link]] and [[#arch]].");
    assert_eq!(refs, vec!["#intro", "#arch"]);
}

#[test]
fn extract_hash_references_preserves_hash_suffix_when_present() {
    let refs = extract_hash_references("See [[#arch-v1@abc123]] and [[#intro]].");
    assert_eq!(refs.len(), 2);
    assert_eq!(refs[0].target_id, "arch-v1");
    assert_eq!(refs[0].expect_hash.as_deref(), Some("abc123"));
    assert_eq!(refs[1].target_id, "intro");
    assert_eq!(refs[1].expect_hash, None);
}

#[test]
fn validate_contract_supports_core_semantic_check_minilanguage() {
    assert!(validate_contract("must_contain(\"Rust\")", "Rust guide").is_none());
    assert!(validate_contract("must_not_contain(\"draft\")", "stable guide").is_none());
    assert!(validate_contract("min_length(20)", "short").is_some());
}

#[test]
fn semantic_check_helpers_extract_function_args_and_ids() {
    assert_eq!(
        extract_function_args("must_contain(\"Rust\", \"Lock\")", "must_contain"),
        Some("\"Rust\", \"Lock\"")
    );
    assert_eq!(
        generate_suggested_id("Architecture Overview!"),
        "architecture-overview"
    );
}
