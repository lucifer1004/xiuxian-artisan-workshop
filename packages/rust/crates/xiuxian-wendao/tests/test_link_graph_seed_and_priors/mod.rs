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
//! Integration tests for structural priors and seed-grounded related retrieval.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use tempfile::TempDir;
use xiuxian_wendao::LinkGraphIndex;
use xiuxian_wendao::link_graph::{
    LinkGraphEdgeType, LinkGraphPprSubgraphMode, LinkGraphRelatedFilter,
    LinkGraphRelatedPprOptions, LinkGraphSearchFilters, LinkGraphSearchOptions,
};

fn write_file(path: &Path, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

mod link_graph_related_filter_seed_accuracy_is_cluster_grounded;
mod link_graph_structural_priors_promote_architecture_hub_top3;
