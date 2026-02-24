use super::super::super::{LinkGraphDocument, LinkGraphIndex, LinkGraphSearchOptions};

impl LinkGraphIndex {
    pub(in crate::link_graph::index::search) fn matches_temporal_filters(
        doc: &LinkGraphDocument,
        options: &LinkGraphSearchOptions,
    ) -> bool {
        if let Some(created_after) = options.created_after
            && doc.created_ts.is_none_or(|ts| ts < created_after)
        {
            return false;
        }
        if let Some(created_before) = options.created_before
            && doc.created_ts.is_none_or(|ts| ts > created_before)
        {
            return false;
        }
        if let Some(modified_after) = options.modified_after
            && doc.modified_ts.is_none_or(|ts| ts < modified_after)
        {
            return false;
        }
        if let Some(modified_before) = options.modified_before
            && doc.modified_ts.is_none_or(|ts| ts > modified_before)
        {
            return false;
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::link_graph::index::search) fn matches_structured_filters(
        &self,
        doc: &LinkGraphDocument,
        options: &LinkGraphSearchOptions,
        include_paths: &[String],
        exclude_paths: &[String],
        tag_all: &[String],
        tag_any: &[String],
        tag_not: &[String],
        mention_filters: &[String],
    ) -> bool {
        if !Self::matches_path_filters(doc, include_paths, exclude_paths) {
            return false;
        }

        if !Self::matches_tag_filters(doc, options, tag_all, tag_any, tag_not) {
            return false;
        }

        if !self.matches_graph_state_filters(doc, options, mention_filters) {
            return false;
        }

        true
    }
}
