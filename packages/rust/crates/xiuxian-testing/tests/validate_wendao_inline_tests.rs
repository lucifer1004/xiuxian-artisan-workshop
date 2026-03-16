//! Validate xiuxian-wendao source files for inline test blocks.
//!
//! This test detects `#[cfg(test)] mod tests { ... }` blocks that should be
//! externalized to `tests/unit/` directory.

use std::path::Path;

use xiuxian_testing::external_test::{detect_inline_test_blocks, format_inline_test_report};

#[test]
fn validate_wendao_no_inline_test_blocks() {
    let wendao_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("xiuxian-wendao");

    if !wendao_root.exists() {
        panic!("xiuxian-wendao not found at {:?}", wendao_root);
    }

    // Detect inline test blocks with minimum 10 lines
    let issues = detect_inline_test_blocks(&wendao_root, 10);

    if !issues.is_empty() {
        let report = format_inline_test_report(&issues);
        panic!(
            "\n{}\n\n❌ Found {} inline test block(s). Externalize them to pass this test.",
            report,
            issues.len()
        );
    } else {
        eprintln!("✅ No inline test blocks found. All tests properly externalized.");
    }
}
