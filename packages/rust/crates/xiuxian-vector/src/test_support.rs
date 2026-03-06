//! Stable test-support APIs for integration harnesses.

use xiuxian_types::VectorSearchResult;

use crate::skill::ToolSearchResult;

/// Exposes the keyword boost coefficient used in search scoring.
#[must_use]
pub fn keyword_boost() -> f32 {
    crate::search_impl::keyword_boost_for_test()
}

/// Encodes vector search results into Arrow IPC bytes.
///
/// # Errors
///
/// Returns an error when projection is invalid or Arrow serialization fails.
pub fn search_results_to_ipc(
    results: &[VectorSearchResult],
    projection: Option<&[String]>,
) -> Result<Vec<u8>, String> {
    crate::search_impl::search_results_to_ipc_for_test(results, projection)
}

/// Encodes tool search results into Arrow IPC bytes.
///
/// # Errors
///
/// Returns an error when Arrow serialization fails.
pub fn tool_search_results_to_ipc(results: &[ToolSearchResult]) -> Result<Vec<u8>, String> {
    crate::search_impl::tool_search_results_to_ipc_for_test(results)
}
