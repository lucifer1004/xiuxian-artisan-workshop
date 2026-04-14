use std::fs;
use std::path::Path;

use walkdir::WalkDir;

use crate::parsers::docs_governance::{
    extract_hidden_path_links, is_canonical_repo_doc, is_package_local_crate_doc,
};
use crate::zhenfa_router::native::semantic_check::docs_governance::scope::scope_matches_path;
use crate::zhenfa_router::native::semantic_check::docs_governance::types::CANONICAL_DOC_HIDDEN_PATH_LINK_ISSUE_TYPE;
use crate::zhenfa_router::native::semantic_check::{IssueLocation, SemanticIssue};

/// Collect hidden workspace-path link issues for one canonical document.
#[must_use]
pub(crate) fn collect_hidden_path_link_issues(doc_path: &str, content: &str) -> Vec<SemanticIssue> {
    if !is_canonical_repo_doc(doc_path) {
        return Vec::new();
    }

    extract_hidden_path_links(content)
        .into_iter()
        .map(|link| SemanticIssue {
            severity: "warning".to_string(),
            issue_type: CANONICAL_DOC_HIDDEN_PATH_LINK_ISSUE_TYPE.to_string(),
            doc: doc_path.to_string(),
            node_id: doc_path.to_string(),
            message: format!(
                "Canonical repository docs must not link hidden workspace paths; found '{}'",
                link.target
            ),
            location: Some(IssueLocation {
                line: link.line,
                heading_path: "Canonical Doc Links".to_string(),
                byte_range: Some((link.start_offset, link.end_offset)),
            }),
            suggestion: Some(format!(
                "Remove `{}` or replace it with a stable RFC, package-doc, README, or other canonical reference. Keep exact hidden tracking paths only in ExecPlans, GTD, or similar task records.",
                link.link_markup
            )),
            fuzzy_suggestion: None,
        })
        .collect()
}

/// Collect hidden workspace-path link issues across non-package canonical docs.
#[must_use]
pub(crate) fn collect_workspace_canonical_doc_issues(
    root: &Path,
    scope: Option<&str>,
) -> Vec<SemanticIssue> {
    let mut issues = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || !entry.file_type().is_dir()
                || !entry.file_name().to_string_lossy().starts_with('.')
        })
        .flatten()
    {
        let path = entry.path();
        if !entry.file_type().is_file() {
            continue;
        }

        let path_str = path.to_string_lossy().to_string();
        if !is_canonical_repo_doc(&path_str)
            || is_package_local_crate_doc(&path_str)
            || !scope_matches_path(scope, path)
        {
            continue;
        }

        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };
        issues.extend(collect_hidden_path_link_issues(&path_str, &content));
    }

    issues
}
