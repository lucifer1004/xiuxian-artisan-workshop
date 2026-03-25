use arrow_string::like;

use crate::{LanceBooleanArray, LanceStringArray, VectorStoreError};

/// Compute a boolean mask for `contains` against one UTF-8 array and one scalar needle.
///
/// # Errors
///
/// Returns an error when Arrow cannot evaluate the vectorized substring predicate.
pub fn string_contains_mask(
    values: &LanceStringArray,
    needle: &str,
) -> Result<LanceBooleanArray, VectorStoreError> {
    let pattern = LanceStringArray::new_scalar(needle);
    like::contains(values, &pattern).map_err(VectorStoreError::Arrow)
}
