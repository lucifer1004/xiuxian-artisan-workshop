#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ParsedRepoCodeSearchQuery {
    pub(crate) language_filters: std::collections::HashSet<String>,
    pub(crate) kind_filters: std::collections::HashSet<String>,
    pub(crate) search_term: Option<String>,
}

impl ParsedRepoCodeSearchQuery {
    pub(crate) fn search_term(&self) -> Option<&str> {
        self.search_term.as_deref()
    }
}

pub(crate) fn parse_repo_code_search_query(query: &str) -> ParsedRepoCodeSearchQuery {
    let mut spec = ParsedRepoCodeSearchQuery::default();
    let mut search_tokens = Vec::new();
    for token in query.split_whitespace() {
        if let Some(value) = token.strip_prefix("lang:") {
            let normalized = value.trim().to_ascii_lowercase();
            if !normalized.is_empty() {
                spec.language_filters.insert(normalized);
            }
            continue;
        }

        if let Some(value) = token.strip_prefix("kind:") {
            let normalized = value.trim().to_ascii_lowercase();
            if matches!(
                normalized.as_str(),
                "file" | "symbol" | "function" | "module" | "example"
            ) {
                spec.kind_filters.insert(normalized);
                continue;
            }
        }

        search_tokens.push(token.to_string());
    }

    spec.search_term = (!search_tokens.is_empty()).then(|| search_tokens.join(" "));
    spec
}
