#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ParsedRepoCodeSearchQuery {
    pub(crate) repo: Option<String>,
    pub(crate) language_filters: std::collections::HashSet<String>,
    pub(crate) kind_filters: std::collections::HashSet<String>,
    pub(crate) ast_pattern: Option<String>,
    pub(crate) search_term: Option<String>,
}

impl ParsedRepoCodeSearchQuery {
    pub(crate) fn search_term(&self) -> Option<&str> {
        self.search_term.as_deref()
    }
}
