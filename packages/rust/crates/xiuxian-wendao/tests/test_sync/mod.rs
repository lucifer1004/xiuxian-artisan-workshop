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
//! Integration tests for Rust SyncEngine

use std::fs;
use tempfile::TempDir;

mod batch_diff_computation;
mod compute_diff;
mod compute_hash;
mod custom_discovery_options;
mod deleted_files_detection;
mod discover_files;
/// Test SyncEngine manifest load/save operations
mod manifest_load_save;
mod skip_hidden_and_directories;
