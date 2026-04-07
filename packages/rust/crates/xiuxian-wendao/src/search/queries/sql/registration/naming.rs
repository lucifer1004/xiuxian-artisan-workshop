use crate::search::{SearchCorpusKind, SearchPlaneService};

pub(super) fn local_sql_table_name(
    corpus: SearchCorpusKind,
    registered_table_name: &str,
) -> String {
    match corpus {
        SearchCorpusKind::LocalSymbol => registered_table_name.to_string(),
        SearchCorpusKind::KnowledgeSection
        | SearchCorpusKind::Attachment
        | SearchCorpusKind::ReferenceOccurrence => corpus.to_string(),
        SearchCorpusKind::RepoEntity | SearchCorpusKind::RepoContentChunk => {
            registered_table_name.to_string()
        }
    }
}

pub(super) fn repo_sql_table_name(corpus: SearchCorpusKind, repo_id: &str) -> String {
    match corpus {
        SearchCorpusKind::RepoEntity => SearchPlaneService::repo_entity_table_name(repo_id),
        SearchCorpusKind::RepoContentChunk => {
            SearchPlaneService::repo_content_chunk_table_name(repo_id)
        }
        SearchCorpusKind::LocalSymbol
        | SearchCorpusKind::KnowledgeSection
        | SearchCorpusKind::Attachment
        | SearchCorpusKind::ReferenceOccurrence => corpus.to_string(),
    }
}

pub(super) fn repo_logical_view_name(corpus: SearchCorpusKind) -> String {
    corpus.to_string()
}

pub(super) fn local_logical_view_name(corpus: SearchCorpusKind) -> String {
    corpus.to_string()
}
