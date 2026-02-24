use super::super::super::{
    LinkGraphDocument, LinkGraphIndex, LinkGraphScope, SectionCandidate, SectionMatch,
};
use super::super::context::SearchExecutionContext;

impl LinkGraphIndex {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::link_graph::index::search) fn prepare_section_context(
        &self,
        doc: &LinkGraphDocument,
        context: &SearchExecutionContext,
        structural_edges_enabled: bool,
        scope: LinkGraphScope,
        per_doc_section_cap: usize,
        min_section_words: usize,
        max_heading_level: usize,
        max_tree_hops: Option<usize>,
    ) -> (Vec<SectionCandidate>, Option<SectionMatch>, f64) {
        let mut section_candidates = if structural_edges_enabled {
            self.section_candidates(
                &doc.id,
                &context.clean_query,
                &context.query_tokens,
                context.case_sensitive,
                max_heading_level,
                min_section_words,
                max_tree_hops,
            )
        } else {
            Vec::new()
        };
        if matches!(scope, LinkGraphScope::SectionOnly | LinkGraphScope::Mixed) {
            section_candidates.retain(|row| !row.heading_path.trim().is_empty());
        }
        if section_candidates.len() > per_doc_section_cap {
            section_candidates.truncate(per_doc_section_cap);
        }
        let section_match = Self::best_section_match(&section_candidates);
        let section_score = section_match.as_ref().map_or(0.0, |row| row.score);
        (section_candidates, section_match, section_score)
    }
}
