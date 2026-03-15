//! Native paths test API for DeepSeek vision module.

use std::path::{Path, PathBuf};

use super::super::model_kind::VisionModelKind;
pub use super::super::native::{DsqRepairResult, repair_dsq_if_needed};

/// Resolve weights path with explicit parameters for test assertions.
pub fn resolve_weights_path_with_for_tests(
    model_root: &Path,
    model_kind: Option<&str>,
    override_path: Option<&str>,
) -> Result<PathBuf, String> {
    let kind = model_kind
        .and_then(VisionModelKind::parse)
        .unwrap_or(VisionModelKind::DEFAULT);
    super::super::native::resolve_weights_path_with_for_tests(model_root, kind, override_path)
}
