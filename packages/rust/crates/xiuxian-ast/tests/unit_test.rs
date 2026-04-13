//! Cargo entry point for `xiuxian-ast` unit tests.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/chunk.rs"]
mod chunk;
#[path = "unit/extract.rs"]
mod extract;
#[path = "unit/lang.rs"]
mod lang;
#[path = "unit/python.rs"]
mod python;
#[path = "unit/python_tree_sitter.rs"]
mod python_tree_sitter;
#[path = "unit/scan.rs"]
mod scan;
#[path = "unit/scan_decorator.rs"]
mod scan_decorator;
#[path = "unit/security.rs"]
mod security;
