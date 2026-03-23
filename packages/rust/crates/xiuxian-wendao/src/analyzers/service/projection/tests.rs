use crate::analyzers::query::{
    DocsProjectedGapReportQuery, DocsSearchQuery, RepoProjectedGapReportQuery,
    RepoProjectedPageIndexTreesQuery, RepoProjectedPageSearchQuery, RepoProjectedPagesQuery,
    RepoProjectedRetrievalQuery,
};

use super::{
    build_docs_projected_gap_report, build_docs_search, build_repo_projected_gap_report,
    build_repo_projected_page_index_trees, build_repo_projected_page_search,
    build_repo_projected_pages, build_repo_projected_retrieval,
};

#[allow(dead_code)]
#[path = "../../../../tests/support/repo_intelligence.rs"]
mod repo_test_support;

#[test]
fn repo_projected_pages_wraps_projection_fixture() {
    let analysis = repo_test_support::sample_projection_analysis("projection-sample");
    let result = build_repo_projected_pages(
        &RepoProjectedPagesQuery {
            repo_id: "projection-sample".to_string(),
        },
        &analysis,
    );

    assert_eq!(result.repo_id, "projection-sample");
    assert!(!result.pages.is_empty());
}

#[test]
fn repo_and_docs_gap_reports_share_the_same_surface() {
    let analysis = repo_test_support::sample_projection_analysis("projection-sample");
    let repo_result = build_repo_projected_gap_report(
        &RepoProjectedGapReportQuery {
            repo_id: "projection-sample".to_string(),
        },
        &analysis,
    );
    let docs_result = build_docs_projected_gap_report(
        &DocsProjectedGapReportQuery {
            repo_id: "projection-sample".to_string(),
        },
        &analysis,
    );

    assert_eq!(repo_result, docs_result);
}

#[test]
fn docs_and_repo_projected_search_results_match() {
    let analysis = repo_test_support::sample_projection_analysis("projection-sample");
    let repo_result = build_repo_projected_page_search(
        &RepoProjectedPageSearchQuery {
            repo_id: "projection-sample".to_string(),
            query: "solve".to_string(),
            kind: None,
            limit: 10,
        },
        &analysis,
    );
    let docs_result = build_docs_search(
        &DocsSearchQuery {
            repo_id: "projection-sample".to_string(),
            query: "solve".to_string(),
            kind: None,
            limit: 10,
        },
        &analysis,
    );

    assert_eq!(repo_result, docs_result);
    assert!(!repo_result.pages.is_empty());
}

#[test]
fn projected_page_index_trees_and_retrieval_wrap_the_fixture() {
    let analysis = repo_test_support::sample_projection_analysis("projection-sample");
    let trees = build_repo_projected_page_index_trees(
        &RepoProjectedPageIndexTreesQuery {
            repo_id: "projection-sample".to_string(),
        },
        &analysis,
    )
    .expect("fixture should parse into projected page-index trees");
    let retrieval = build_repo_projected_retrieval(
        &RepoProjectedRetrievalQuery {
            repo_id: "projection-sample".to_string(),
            query: "solve".to_string(),
            kind: None,
            limit: 10,
        },
        &analysis,
    );

    assert_eq!(trees.repo_id, "projection-sample");
    assert!(!trees.trees.is_empty());
    assert_eq!(retrieval.repo_id, "projection-sample");
    assert!(!retrieval.hits.is_empty());
}
