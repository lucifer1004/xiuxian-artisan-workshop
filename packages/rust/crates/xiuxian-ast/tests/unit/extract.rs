//! Package-top harness for extract unit tests.

use xiuxian_ast::{
    Lang, extract_items, extract_skeleton, get_skeleton_patterns, semantic_fingerprint,
};

#[path = "extract_tests.rs"]
mod extract_tests;
