use std::path::{Path, PathBuf};

use crate::analyzers::errors::RepoIntelligenceError;
#[cfg(feature = "zhenfa-router")]
use crate::analyzers::plugin::RepositoryAnalysisOutput;
use crate::analyzers::projection::{ProjectedPageIndexNode, ProjectionPageKind};
use crate::analyzers::query::{
    DocsMarkdownDocumentsQuery, DocsNavigationQuery, DocsNavigationResult,
    DocsPageIndexDocumentsQuery, DocsPageIndexDocumentsResult, DocsPageIndexNodeQuery,
    DocsPageIndexNodeResult, DocsPageIndexTreeQuery, DocsPageIndexTreeResult,
    DocsPageIndexTreeSearchQuery, DocsPageIndexTreeSearchResult, DocsPageIndexTreesQuery,
    DocsPageIndexTreesResult, DocsPageQuery, DocsPageResult, DocsRetrievalContextQuery,
    DocsRetrievalContextResult,
};
#[cfg(feature = "zhenfa-router")]
use crate::analyzers::registry::PluginRegistry;
#[cfg(feature = "zhenfa-router")]
use crate::analyzers::service::projection::{
    build_docs_navigation, build_docs_page, build_docs_page_index_tree,
    build_docs_retrieval_context,
};
use crate::analyzers::service::projection::{
    docs_markdown_documents_from_config, docs_navigation_from_config, docs_page_from_config,
    docs_page_index_documents_from_config, docs_page_index_node_from_config,
    docs_page_index_tree_from_config, docs_page_index_tree_search_from_config,
    docs_page_index_trees_from_config, docs_retrieval_context_from_config,
};
#[cfg(feature = "zhenfa-router")]
use crate::analyzers::{RegisteredRepository, analyze_registered_repository_with_registry};

use super::{
    DocsDocumentSegmentResult, DocsNavigationOptions, DocsRetrievalContextOptions,
    build_document_segment,
};

/// Crate-local capability facade for docs/page-index operations.
///
/// This service stays parallel to `SearchQueryService`: it owns the in-process
/// docs capability surface, while gateway and CLI surfaces act as adapters.
#[derive(Clone, Debug)]
pub struct DocsToolService {
    project_root: PathBuf,
    repo_id: String,
    config_path: Option<PathBuf>,
}

impl DocsToolService {
    #[cfg(feature = "zhenfa-router")]
    fn with_registered_repository_analysis<T, F>(
        &self,
        repository: &RegisteredRepository,
        registry: &PluginRegistry,
        build: F,
    ) -> Result<T, RepoIntelligenceError>
    where
        F: FnOnce(&RepositoryAnalysisOutput) -> Result<T, RepoIntelligenceError>,
    {
        let analysis =
            analyze_registered_repository_with_registry(repository, self.project_root(), registry)?;
        build(&analysis)
    }

    /// Create a docs capability service for one project root and repository.
    #[must_use]
    pub fn new(project_root: impl Into<PathBuf>, repo_id: impl Into<String>) -> Self {
        Self {
            project_root: project_root.into(),
            repo_id: repo_id.into(),
            config_path: None,
        }
    }

    /// Create a docs capability service from one project root.
    #[must_use]
    pub fn from_project_root(project_root: impl Into<PathBuf>, repo_id: impl Into<String>) -> Self {
        Self::new(project_root, repo_id)
    }

    /// Override the config path used by config-backed docs capability calls.
    #[must_use]
    pub fn with_optional_config_path(mut self, config_path: Option<PathBuf>) -> Self {
        self.config_path = config_path;
        self
    }

    /// Borrow the project root used for capability calls.
    #[must_use]
    pub fn project_root(&self) -> &Path {
        self.project_root.as_path()
    }

    /// Borrow the repository identifier used for capability calls.
    #[must_use]
    pub fn repo_id(&self) -> &str {
        self.repo_id.as_str()
    }

    /// Borrow the optional config path used for capability calls.
    #[must_use]
    pub fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }

    /// Return one deterministic docs-facing projected page.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when repository analysis fails or the
    /// requested projected page is not present for the configured repository.
    pub fn get_document(&self, page_id: &str) -> Result<DocsPageResult, RepoIntelligenceError> {
        docs_page_from_config(
            &DocsPageQuery {
                repo_id: self.repo_id.clone(),
                page_id: page_id.to_string(),
            },
            self.config_path(),
            self.project_root(),
        )
    }

    #[cfg(feature = "zhenfa-router")]
    pub(crate) fn get_document_for_registered_repository(
        &self,
        page_id: &str,
        repository: &RegisteredRepository,
        registry: &PluginRegistry,
    ) -> Result<DocsPageResult, RepoIntelligenceError> {
        self.with_registered_repository_analysis(repository, registry, |analysis| {
            build_docs_page(
                &DocsPageQuery {
                    repo_id: repository.id.clone(),
                    page_id: page_id.to_string(),
                },
                analysis,
            )
        })
    }

    /// Return one deterministic docs-facing projected page-index tree.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when repository analysis fails, the
    /// requested page is not present, or page-index tree construction fails.
    pub fn get_document_structure(
        &self,
        page_id: &str,
    ) -> Result<DocsPageIndexTreeResult, RepoIntelligenceError> {
        docs_page_index_tree_from_config(
            &DocsPageIndexTreeQuery {
                repo_id: self.repo_id.clone(),
                page_id: page_id.to_string(),
            },
            self.config_path(),
            self.project_root(),
        )
    }

    /// Return one text-free docs-facing projected page-index tree for
    /// token-sensitive structure inspection.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when repository analysis fails, the
    /// requested page is not present, or page-index tree construction fails.
    pub fn get_document_structure_outline(
        &self,
        page_id: &str,
    ) -> Result<DocsPageIndexTreeResult, RepoIntelligenceError> {
        self.get_document_structure(page_id)
            .map(text_free_tree_result)
    }

    /// Return one repo-scoped text-free docs-facing projected page-index tree
    /// catalog for token-sensitive structure discovery.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when repository analysis fails or
    /// page-index tree construction fails.
    pub fn get_document_structure_catalog(
        &self,
    ) -> Result<DocsPageIndexTreesResult, RepoIntelligenceError> {
        docs_page_index_trees_from_config(
            &DocsPageIndexTreesQuery {
                repo_id: self.repo_id.clone(),
            },
            self.config_path(),
            self.project_root(),
        )
        .map(text_free_trees_result)
    }

    /// Return one precise docs-facing projected markdown segment reopened by a
    /// stable page id plus 1-based inclusive line range.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when repository analysis fails, the
    /// requested projected page is not present, or the requested line range is
    /// invalid for the rendered projected markdown document.
    pub fn get_document_segment(
        &self,
        page_id: &str,
        line_start: usize,
        line_end: usize,
    ) -> Result<DocsDocumentSegmentResult, RepoIntelligenceError> {
        let documents = docs_markdown_documents_from_config(
            &DocsMarkdownDocumentsQuery {
                repo_id: self.repo_id.clone(),
            },
            self.config_path(),
            self.project_root(),
        )?;
        let document = documents
            .documents
            .iter()
            .find(|document| document.page_id == page_id)
            .ok_or_else(|| RepoIntelligenceError::UnknownProjectedPage {
                repo_id: self.repo_id.clone(),
                page_id: page_id.to_string(),
            })?;
        build_document_segment(document, line_start, line_end)
    }

    /// Return one deterministic docs-facing projected page-index node.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when repository analysis fails, the
    /// requested page is not present, or the requested page-index node is not
    /// present for the projected page.
    pub fn get_document_node(
        &self,
        page_id: &str,
        node_id: &str,
    ) -> Result<DocsPageIndexNodeResult, RepoIntelligenceError> {
        docs_page_index_node_from_config(
            &DocsPageIndexNodeQuery {
                repo_id: self.repo_id.clone(),
                page_id: page_id.to_string(),
                node_id: node_id.to_string(),
            },
            self.config_path(),
            self.project_root(),
        )
    }

    /// Search docs-facing page-index nodes across one repository and return
    /// bounded deterministic candidate hits.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when repository analysis fails or
    /// projected page-index tree search construction fails.
    pub fn search_document_structure(
        &self,
        query: &str,
        kind: Option<ProjectionPageKind>,
        limit: usize,
    ) -> Result<DocsPageIndexTreeSearchResult, RepoIntelligenceError> {
        docs_page_index_tree_search_from_config(
            &DocsPageIndexTreeSearchQuery {
                repo_id: self.repo_id.clone(),
                query: query.to_string(),
                kind,
                limit: limit.max(1),
            },
            self.config_path(),
            self.project_root(),
        )
    }

    #[cfg(feature = "zhenfa-router")]
    pub(crate) fn get_document_structure_for_registered_repository(
        &self,
        page_id: &str,
        repository: &RegisteredRepository,
        registry: &PluginRegistry,
    ) -> Result<DocsPageIndexTreeResult, RepoIntelligenceError> {
        self.with_registered_repository_analysis(repository, registry, |analysis| {
            build_docs_page_index_tree(
                &DocsPageIndexTreeQuery {
                    repo_id: repository.id.clone(),
                    page_id: page_id.to_string(),
                },
                analysis,
            )
        })
    }

    /// Return repository-scoped markdown TOC/page-index documents for the
    /// configured repository.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when repository analysis fails or
    /// projected markdown cannot be parsed into page-index-ready documents.
    pub fn get_toc_documents(&self) -> Result<DocsPageIndexDocumentsResult, RepoIntelligenceError> {
        docs_page_index_documents_from_config(
            &DocsPageIndexDocumentsQuery {
                repo_id: self.repo_id.clone(),
            },
            self.config_path(),
            self.project_root(),
        )
    }

    /// Return one deterministic docs-facing navigation bundle using default
    /// navigation limits.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when repository analysis fails or the
    /// requested projected page, node, or family cluster is not present.
    pub fn get_navigation(
        &self,
        page_id: &str,
        node_id: Option<&str>,
    ) -> Result<DocsNavigationResult, RepoIntelligenceError> {
        self.get_navigation_with_options(
            page_id,
            DocsNavigationOptions {
                node_id: node_id.map(str::to_string),
                ..DocsNavigationOptions::default()
            },
        )
    }

    /// Return one deterministic docs-facing navigation bundle using explicit
    /// navigation options.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when repository analysis fails or the
    /// requested projected page, node, or family cluster is not present.
    pub fn get_navigation_with_options(
        &self,
        page_id: &str,
        options: DocsNavigationOptions,
    ) -> Result<DocsNavigationResult, RepoIntelligenceError> {
        let options = options.normalized();
        docs_navigation_from_config(
            &DocsNavigationQuery {
                repo_id: self.repo_id.clone(),
                page_id: page_id.to_string(),
                node_id: options.node_id,
                family_kind: options.family_kind,
                related_limit: options.related_limit,
                family_limit: options.family_limit,
            },
            self.config_path(),
            self.project_root(),
        )
    }

    #[cfg(feature = "zhenfa-router")]
    pub(crate) fn get_navigation_with_options_for_registered_repository(
        &self,
        page_id: &str,
        repository: &RegisteredRepository,
        registry: &PluginRegistry,
        options: DocsNavigationOptions,
    ) -> Result<DocsNavigationResult, RepoIntelligenceError> {
        let options = options.normalized();
        self.with_registered_repository_analysis(repository, registry, |analysis| {
            build_docs_navigation(
                &DocsNavigationQuery {
                    repo_id: repository.id.clone(),
                    page_id: page_id.to_string(),
                    node_id: options.node_id,
                    family_kind: options.family_kind,
                    related_limit: options.related_limit,
                    family_limit: options.family_limit,
                },
                analysis,
            )
        })
    }

    /// Return one deterministic docs-facing retrieval context using default
    /// related-page limits.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when repository analysis fails or the
    /// requested projected page or node is not present.
    pub fn get_retrieval_context(
        &self,
        page_id: &str,
        node_id: Option<&str>,
    ) -> Result<DocsRetrievalContextResult, RepoIntelligenceError> {
        self.get_retrieval_context_with_options(
            page_id,
            DocsRetrievalContextOptions {
                node_id: node_id.map(str::to_string),
                ..DocsRetrievalContextOptions::default()
            },
        )
    }

    /// Return one deterministic docs-facing retrieval context using explicit
    /// context options.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when repository analysis fails or the
    /// requested projected page or node is not present.
    pub fn get_retrieval_context_with_options(
        &self,
        page_id: &str,
        options: DocsRetrievalContextOptions,
    ) -> Result<DocsRetrievalContextResult, RepoIntelligenceError> {
        docs_retrieval_context_from_config(
            &DocsRetrievalContextQuery {
                repo_id: self.repo_id.clone(),
                page_id: page_id.to_string(),
                node_id: options.node_id,
                related_limit: options.related_limit,
            },
            self.config_path(),
            self.project_root(),
        )
    }

    #[cfg(feature = "zhenfa-router")]
    pub(crate) fn get_retrieval_context_with_options_for_registered_repository(
        &self,
        page_id: &str,
        repository: &RegisteredRepository,
        registry: &PluginRegistry,
        options: DocsRetrievalContextOptions,
    ) -> Result<DocsRetrievalContextResult, RepoIntelligenceError> {
        self.with_registered_repository_analysis(repository, registry, |analysis| {
            build_docs_retrieval_context(
                &DocsRetrievalContextQuery {
                    repo_id: repository.id.clone(),
                    page_id: page_id.to_string(),
                    node_id: options.node_id,
                    related_limit: options.related_limit,
                },
                analysis,
            )
        })
    }
}

fn text_free_tree_result(mut result: DocsPageIndexTreeResult) -> DocsPageIndexTreeResult {
    if let Some(tree) = result.tree.as_mut() {
        strip_text_from_nodes(tree.roots.as_mut_slice());
    }
    result
}

fn text_free_trees_result(mut result: DocsPageIndexTreesResult) -> DocsPageIndexTreesResult {
    for tree in &mut result.trees {
        strip_text_from_nodes(tree.roots.as_mut_slice());
    }
    result
}

fn strip_text_from_nodes(nodes: &mut [ProjectedPageIndexNode]) {
    for node in nodes {
        node.text.clear();
        strip_text_from_nodes(node.children.as_mut_slice());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::projection::{ProjectedPageIndexTree, ProjectionPageKind};

    #[test]
    fn docs_tool_service_starts_without_config_override() {
        let service = DocsToolService::from_project_root("/tmp/project", "repo-a");

        assert_eq!(service.project_root(), Path::new("/tmp/project"));
        assert_eq!(service.repo_id(), "repo-a");
        assert_eq!(service.config_path(), None);
    }

    #[test]
    fn docs_tool_service_accepts_optional_config_override() {
        let config_path = PathBuf::from("/tmp/project/wendao.toml");
        let service = DocsToolService::from_project_root("/tmp/project", "repo-a")
            .with_optional_config_path(Some(config_path.clone()));

        assert_eq!(service.config_path(), Some(config_path.as_path()));
    }

    #[test]
    fn navigation_options_default_to_docs_limits() {
        let options = DocsNavigationOptions::default();

        assert_eq!(options.related_limit, 5);
        assert_eq!(options.family_limit, 3);
        assert_eq!(options.node_id, None);
        assert_eq!(options.family_kind, None);
    }

    #[test]
    fn navigation_options_normalize_zero_family_limit() {
        let options = DocsNavigationOptions {
            family_kind: Some(ProjectionPageKind::HowTo),
            family_limit: 0,
            ..DocsNavigationOptions::default()
        }
        .normalized();

        assert_eq!(options.family_limit, 1);
        assert_eq!(options.family_kind, Some(ProjectionPageKind::HowTo));
    }

    #[test]
    fn retrieval_context_options_default_to_docs_limit() {
        let options = DocsRetrievalContextOptions::default();

        assert_eq!(options.related_limit, 5);
        assert_eq!(options.node_id, None);
    }

    #[test]
    fn text_free_tree_result_clears_node_text_recursively() {
        let result = DocsPageIndexTreeResult {
            repo_id: "repo-a".to_string(),
            tree: Some(ProjectedPageIndexTree {
                repo_id: "repo-a".to_string(),
                page_id: "page-a".to_string(),
                kind: ProjectionPageKind::Reference,
                path: "reference/page-a.md".to_string(),
                doc_id: "doc:page-a".to_string(),
                title: "Page A".to_string(),
                root_count: 1,
                roots: vec![ProjectedPageIndexNode {
                    node_id: "n1".to_string(),
                    title: "Root".to_string(),
                    level: 1,
                    structural_path: vec!["Root".to_string()],
                    line_range: (1, 3),
                    token_count: 3,
                    is_thinned: false,
                    text: "root body".to_string(),
                    summary: Some("summary".to_string()),
                    children: vec![ProjectedPageIndexNode {
                        node_id: "n2".to_string(),
                        title: "Child".to_string(),
                        level: 2,
                        structural_path: vec!["Root".to_string(), "Child".to_string()],
                        line_range: (2, 3),
                        token_count: 2,
                        is_thinned: false,
                        text: "child body".to_string(),
                        summary: Some("child summary".to_string()),
                        children: Vec::new(),
                    }],
                }],
            }),
        };

        let stripped = text_free_tree_result(result);
        let roots = stripped.tree.unwrap().roots;
        assert_eq!(roots[0].text, "");
        assert_eq!(roots[0].summary.as_deref(), Some("summary"));
        assert_eq!(roots[0].children[0].text, "");
        assert_eq!(
            roots[0].children[0].summary.as_deref(),
            Some("child summary")
        );
    }

    #[test]
    fn text_free_trees_result_clears_node_text_recursively() {
        let result = DocsPageIndexTreesResult {
            repo_id: "repo-a".to_string(),
            trees: vec![ProjectedPageIndexTree {
                repo_id: "repo-a".to_string(),
                page_id: "page-a".to_string(),
                kind: ProjectionPageKind::Reference,
                path: "reference/page-a.md".to_string(),
                doc_id: "doc:page-a".to_string(),
                title: "Page A".to_string(),
                root_count: 1,
                roots: vec![ProjectedPageIndexNode {
                    node_id: "n1".to_string(),
                    title: "Root".to_string(),
                    level: 1,
                    structural_path: vec!["Root".to_string()],
                    line_range: (1, 3),
                    token_count: 3,
                    is_thinned: false,
                    text: "root body".to_string(),
                    summary: Some("summary".to_string()),
                    children: vec![ProjectedPageIndexNode {
                        node_id: "n2".to_string(),
                        title: "Child".to_string(),
                        level: 2,
                        structural_path: vec!["Root".to_string(), "Child".to_string()],
                        line_range: (2, 3),
                        token_count: 2,
                        is_thinned: false,
                        text: "child body".to_string(),
                        summary: Some("child summary".to_string()),
                        children: Vec::new(),
                    }],
                }],
            }],
        };

        let stripped = text_free_trees_result(result);
        let roots = &stripped.trees[0].roots;
        assert_eq!(roots[0].text, "");
        assert_eq!(roots[0].summary.as_deref(), Some("summary"));
        assert_eq!(roots[0].children[0].text, "");
        assert_eq!(
            roots[0].children[0].summary.as_deref(),
            Some("child summary")
        );
    }
}
