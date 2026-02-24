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
//! Benchmark tests for Cargo.toml parsing performance.
//!
//! These tests measure the performance of parsing Cargo.toml files
//! for dependency extraction.

use std::io::Write as StdWrite;
use tempfile::NamedTempFile;
use xiuxian_wendao::dependency_indexer::parse_cargo_dependencies;

/// Generate a complex Cargo.toml for benchmarking.
fn generate_cargo_toml(dep_count: usize) -> String {
    let mut content = String::from(
        "[package]\nname = \"test-crate\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
    );

    // Add simple dependencies
    for i in 0..dep_count {
        content.push_str(&format!(
            "dep{} = \"{}.{}.{}\"\n",
            i,
            i / 100,
            (i / 10) % 10,
            i % 10
        ));
    }

    content.push_str("\n[dev-dependencies]\n");
    for i in 0..(dep_count / 3) {
        content.push_str(&format!(
            "dev_dep{} = \"{}.{}.{}\"\n",
            i,
            i / 100,
            (i / 10) % 10,
            i % 10
        ));
    }

    content
}

/// Generate a workspace Cargo.toml for benchmarking.
fn generate_workspace_cargo_toml(member_count: usize, dep_count: usize) -> String {
    let mut content = String::from("[workspace]\nmembers = [");

    // Add workspace members
    for i in 0..member_count {
        content.push_str(&format!("\"crate{}\", ", i));
    }
    content.push_str("]\n\n[workspace.dependencies]\n");

    // Add workspace dependencies with complex format
    for i in 0..dep_count {
        content.push_str(&format!(
            "dep{} = {{ version = \"{}.{}.{}\", features = [\"feature-a\", \"feature-b\"] }}\n",
            i,
            i / 100,
            (i / 10) % 10,
            i % 10
        ));
    }

    // Add simple dependencies
    for i in 0..(dep_count / 2) {
        content.push_str(&format!(
            "simple_dep{} = \"{}.{}.{}\"\n",
            i,
            i / 100,
            (i / 10) % 10,
            i % 10
        ));
    }

    content
}

/// Benchmark test for parsing Cargo.toml with many dependencies.
mod cargo_toml_parsing_performance;
mod complex_dependency_parsing_performance;
mod parsing_vs_io_overhead;
mod workspace_cargo_toml_parsing_performance;
