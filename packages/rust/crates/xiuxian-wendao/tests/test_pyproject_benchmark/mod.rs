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
//! Benchmark tests for pyproject.toml parsing performance.
//!
//! These tests measure the performance of parsing pyproject.toml files
//! for Python dependency extraction.

use std::io::Write as StdWrite;
use tempfile::NamedTempFile;
use xiuxian_wendao::dependency_indexer::parse_pyproject_dependencies;

/// Generate a pyproject.toml with many dependencies.
fn generate_pyproject_toml(dep_count: usize) -> String {
    let mut content = String::from(
        r#"[project]
name = "test-project"
version = "0.1.0"
description = "A test project"
requires-python = ">=3.10"
dependencies = [
"#,
    );

    // Add dependencies with various version specifiers
    for i in 0..dep_count {
        content.push_str(&format!(
            "    \"package{}=={}.{}.{}\",\n",
            i,
            i / 100,
            (i / 10) % 10,
            i % 10
        ));
    }

    content.push_str("]\n\n[project.optional-dependencies]\ndev = [\n");
    for i in 0..(dep_count / 3) {
        content.push_str(&format!("    \"dev_package{}>=1.0.0\",\n", i));
    }
    content.push_str("]\n");

    content
}

/// Generate a complex pyproject.toml with extras.
fn generate_pyproject_toml_with_extras(dep_count: usize) -> String {
    let mut content = String::from(
        r#"[project]
name = "test-project"
version = "0.1.0"
dependencies = [
"#,
    );

    // Add dependencies with extras (e.g., package[extra]==version)
    for i in 0..dep_count {
        let extra = if i % 5 == 0 {
            "ssl"
        } else if i % 5 == 1 {
            "cli"
        } else if i % 5 == 2 {
            "dev"
        } else {
            "full"
        };
        content.push_str(&format!(
            "    \"package{}[{}]=={}.{}.{}\",\n",
            i,
            extra,
            i / 100,
            (i / 10) % 10,
            i % 10
        ));
    }

    content.push_str("]\n");
    content
}

mod minimal_pyproject_parsing_performance;
mod mixed_pyproject_parsing_performance;
mod pyproject_extras_parsing_performance;
/// Benchmark test for parsing pyproject.toml with many dependencies.
mod pyproject_parsing_performance;
mod regex_fallback_parsing_performance;
