use xiuxian_wendao::repo_intelligence::{DocRecord, RepoSymbolKind};

#[derive(Debug, Clone)]
pub(crate) struct CollectedDoc {
    pub(crate) record: DocRecord,
    pub(crate) target_ids: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct ParsedDeclaration {
    pub(crate) name: String,
    pub(crate) kind: RepoSymbolKind,
    pub(crate) signature: String,
}
