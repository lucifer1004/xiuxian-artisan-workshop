//! Native paths test API for `DeepSeek` vision module.

use std::path::{Path, PathBuf};

use crate::llm::vision::deepseek::model_kind::VisionModelKind;
use crate::llm::vision::deepseek::native;
pub use crate::llm::vision::deepseek::native::{DsqRepairResult, repair_dsq_if_needed};

/// Resolve weights path with explicit parameters for test assertions.
///
/// # Errors
///
/// Returns an error when the requested model kind or weight layout cannot be
/// resolved into a concrete on-disk weights file.
pub fn resolve_weights_path_with_for_tests(
    model_root: &Path,
    model_kind: Option<&str>,
    override_path: Option<&str>,
) -> Result<PathBuf, String> {
    let kind = model_kind
        .and_then(VisionModelKind::parse)
        .unwrap_or(VisionModelKind::DEFAULT);
    native::resolve_weights_path_with_for_tests(model_root, kind, override_path)
}

/// Resolve snapshot path with explicit parameters for test assertions.
#[must_use]
pub fn resolve_snapshot_path_with_for_tests(
    model_root: &Path,
    override_path: Option<&str>,
) -> Option<PathBuf> {
    let override_path = override_path.map(Path::new);
    native::resolve_snapshot_path_with_for_tests(model_root, override_path)
}
