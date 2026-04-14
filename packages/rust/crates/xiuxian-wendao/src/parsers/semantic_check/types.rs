//! Parser-owned semantic-check helper types.

/// A reference with an optional expected hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashReference {
    /// Target ID without the `#` prefix.
    pub target_id: String,
    /// Expected content hash if specified via `@hash`.
    pub expect_hash: Option<String>,
}
