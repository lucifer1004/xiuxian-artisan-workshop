mod search;

#[cfg(test)]
#[path = "../../../../tests/unit/search/knowledge_section/query/mod.rs"]
mod tests;

pub(crate) use search::{KnowledgeSectionSearchError, search_knowledge_sections};
