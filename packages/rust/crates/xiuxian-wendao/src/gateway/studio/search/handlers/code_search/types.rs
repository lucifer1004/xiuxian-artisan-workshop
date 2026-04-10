#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ParsedCodeSearchQuery {
    pub(crate) query: String,
    pub(crate) repo: Option<String>,
    pub(crate) languages: Vec<String>,
    pub(crate) kinds: Vec<String>,
}
