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
//! Benchmark tests for symbols extraction performance.
//!
//! These tests measure the performance of symbol extraction from Rust and Python
//! source files. They are designed to be run with `cargo test` and validate
//! that symbol extraction completes within acceptable time limits.

use std::io::Write as IoWrite;
use std::path::PathBuf;
use tempfile::NamedTempFile;

use xiuxian_wendao::SymbolIndex;
use xiuxian_wendao::dependency_indexer::{ExternalSymbol, SymbolKind, extract_symbols};

/// Generate a large Rust source file for benchmarking.
fn generate_rust_test_file(line_count: usize) -> String {
    let mut content = String::with_capacity(line_count * 50);

    // Add structs
    for i in 0..(line_count / 50) {
        content.push_str(&format!(
            r#"pub struct Struct{} {{
    field_{}: String,
    field_{}: i32,
}}
"#,
            i, i, i
        ));
    }

    // Add enums
    for i in 0..(line_count / 100) {
        content.push_str(&format!(
            r#"pub enum Enum{} {{
    VariantA,
    VariantB(i32),
    VariantC {{ x: i32, y: i32 }},
}}
"#,
            i
        ));
    }

    // Add functions
    for i in 0..(line_count / 30) {
        content.push_str(&format!(
            r#"pub fn function_{}(arg1: &str, arg2: i32) -> Result<(), Box<dyn std::error::Error>> {{
    let _result = process_data(arg1, arg2);
    Ok(())
}}
"#,
            i
        ));
    }

    // Add traits
    for i in 0..(line_count / 80) {
        content.push_str(&format!(
            r#"pub trait Trait{} {{
    fn method_a(&self) -> i32;
    fn method_b(&self, x: i32) -> bool;
}}
"#,
            i
        ));
    }

    content
}

/// Generate a large Python source file for benchmarking.
fn generate_python_test_file(line_count: usize) -> String {
    let mut content = String::with_capacity(line_count * 40);

    // Add classes
    for i in 0..(line_count / 50) {
        content.push_str(&format!(
            r#"class Class{}:
    def __init__(self, param_a: str, param_b: int):
        self.param_a = param_a
        self.param_b = param_b

    def method_a(self) -> str:
        return self.param_a.upper()

    def method_b(self, value: int) -> bool:
        return value > 0

    async def async_method(self) -> dict:
        return {{"status": "ok"}}
"#,
            i
        ));
    }

    // Add functions
    for i in 0..(line_count / 20) {
        content.push_str(&format!(
            r#"def function_{}(arg1: str, arg2: int) -> bool:
    """Process data and return result."""
    result = process(arg1, arg2)
    return result

async def async_function_{}(data: dict) -> list:
    """Async data processing."""
    results = []
    return results
"#,
            i, i
        ));
    }

    content
}

mod mixed_symbol_extraction_performance;
mod python_symbol_extraction_performance;
/// Benchmark test for Rust symbol extraction.
mod rust_symbol_extraction_performance;
mod symbol_index_search_performance;
