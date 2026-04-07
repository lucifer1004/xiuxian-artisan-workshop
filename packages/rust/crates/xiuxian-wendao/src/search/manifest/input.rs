#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchRepoPublicationInput {
    pub(crate) table_name: String,
    pub(crate) schema_version: u32,
    pub(crate) source_revision: Option<String>,
    pub(crate) table_version_id: u64,
    pub(crate) row_count: u64,
    pub(crate) fragment_count: u64,
    pub(crate) published_at: String,
}
