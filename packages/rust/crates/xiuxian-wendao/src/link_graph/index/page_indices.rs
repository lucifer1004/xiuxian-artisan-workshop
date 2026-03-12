use super::LinkGraphIndex;
use crate::link_graph::models::PageIndexNode;
use crate::link_graph::page_index::{
    DEFAULT_PAGE_INDEX_THINNING_TOKEN_THRESHOLD, build_page_index_tree, thin_page_index_tree,
};

impl LinkGraphIndex {
    /// Return the hierarchical `PageIndex` roots for a note.
    #[must_use]
    pub fn page_index(&self, stem_or_id: &str) -> Option<&[PageIndexNode]> {
        let doc_id = self.resolve_doc_id(stem_or_id)?;
        self.get_tree(doc_id).map(Vec::as_slice)
    }

    /// Render the canonical traceability label for one anchor id.
    #[must_use]
    pub fn page_index_trace_label(&self, anchor_id: &str) -> Option<String> {
        self.extract_lineage(anchor_id)
            .map(|path| format!("[Path: {}]", path.join(" > ")))
    }

    #[allow(dead_code)]
    pub(super) fn rebuild_all_page_indices(&mut self) {
        self.trees_by_doc.clear();
        self.node_parent_map.clear();
        let doc_ids = self.docs_by_id.keys().cloned().collect::<Vec<_>>();
        for doc_id in doc_ids {
            self.rebuild_page_index_for_doc(&doc_id);
        }
    }

    pub(in crate::link_graph::index) fn rebuild_page_index_for_doc(&mut self, doc_id: &str) {
        let Some(doc_title) = self.docs_by_id.get(doc_id).map(|doc| doc.title.clone()) else {
            self.remove_page_index_for_doc(doc_id);
            return;
        };
        let Some(sections) = self.sections_by_doc.get(doc_id).cloned() else {
            self.remove_page_index_for_doc(doc_id);
            return;
        };

        self.remove_page_index_for_doc(doc_id);
        let mut tree = build_page_index_tree(doc_id, &doc_title, &sections);
        thin_page_index_tree(&mut tree, DEFAULT_PAGE_INDEX_THINNING_TOKEN_THRESHOLD);
        if tree.is_empty() {
            self.remove_page_index_for_doc(doc_id);
        } else {
            self.index_page_index_nodes(&tree, None);
            self.trees_by_doc.insert(doc_id.to_string(), tree);
        }
    }

    pub(in crate::link_graph::index) fn remove_page_index_for_doc(&mut self, doc_id: &str) {
        self.trees_by_doc.remove(doc_id);
        let prefix = format!("{doc_id}#");
        self.node_parent_map
            .retain(|node_id, _| !node_id.starts_with(&prefix));
    }

    fn index_page_index_nodes(&mut self, nodes: &[PageIndexNode], parent_id: Option<&str>) {
        for node in nodes {
            self.node_parent_map
                .insert(node.node_id.clone(), parent_id.map(str::to_string));
            self.index_page_index_nodes(&node.children, Some(node.node_id.as_str()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::LinkGraphIndex;

    #[test]
    fn rebuild_page_index_populates_node_parent_map_with_root_none_and_child_parent_ids()
    -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempfile::TempDir::new()?;
        let docs_dir = temp_dir.path().join("docs");
        std::fs::create_dir_all(&docs_dir)?;
        std::fs::write(
            docs_dir.join("alpha.md"),
            concat!(
                "# Alpha\n\n",
                "alpha root section carries enough words to stay stable and avoid thinning.\n\n",
                "## Beta\n\n",
                "beta child section carries enough words to keep the nested page index path.\n\n",
                "### Gamma\n\n",
                "gamma leaf section is the nested anchor we use for lineage validation.\n",
            ),
        )?;

        let index = LinkGraphIndex::build(temp_dir.path()).map_err(std::io::Error::other)?;
        let roots = index.page_index("alpha").ok_or("missing page index")?;
        let root = roots.first().ok_or("missing root node")?;
        let beta = root.children.first().ok_or("missing beta node")?;
        let gamma = beta.children.first().ok_or("missing gamma node")?;

        assert_eq!(
            index.get_node_parent_map().get(root.node_id.as_str()),
            Some(&None)
        );
        assert_eq!(
            index.get_node_parent_map().get(beta.node_id.as_str()),
            Some(&Some(root.node_id.clone()))
        );
        assert_eq!(
            index.get_node_parent_map().get(gamma.node_id.as_str()),
            Some(&Some(beta.node_id.clone()))
        );
        assert_eq!(
            index.page_index_semantic_path(gamma.node_id.as_str()),
            Some(vec![
                "Alpha".to_string(),
                "Beta".to_string(),
                "Gamma".to_string(),
            ])
        );

        Ok(())
    }
}
