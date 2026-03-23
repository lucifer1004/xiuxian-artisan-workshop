use std::collections::HashSet;
use std::path::Path;

use walkdir::WalkDir;
use xiuxian_ast::{Lang, extract_items, get_skeleton_patterns};

use crate::gateway::studio::types::{AstSearchHit, UiProjectConfig};

use super::super::project_scope::{configured_project_scan_roots, index_path_for_entry};
use super::super::support::{first_signature_line, infer_crate_name};
use super::filters::{is_markdown_path, should_skip_entry};
use super::markdown::{build_markdown_ast_hits, markdown_scope_name};
use super::navigation::ast_navigation_target;

pub(crate) fn build_ast_index(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> Vec<AstSearchHit> {
    let mut hits = Vec::new();
    let mut seen = HashSet::new();

    for root in configured_project_scan_roots(config_root, projects) {
        for entry in WalkDir::new(root.as_path())
            .into_iter()
            .filter_entry(|entry| !should_skip_entry(entry))
        {
            let Ok(entry) = entry else { continue };
            if !entry.file_type().is_file() {
                continue;
            }

            let normalized_path = index_path_for_entry(project_root, entry.path());
            let normalized_path_ref = Path::new(normalized_path.as_str());
            if is_markdown_path(normalized_path_ref) {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    let crate_name = markdown_scope_name(normalized_path_ref);

                    for hit in build_markdown_ast_hits(
                        root.as_path(),
                        entry.path(),
                        normalized_path.as_str(),
                        content.as_str(),
                        crate_name.as_str(),
                    ) {
                        let dedupe_key = format!(
                            "{}:{}:{}:{}",
                            hit.path, hit.line_start, hit.line_end, hit.name
                        );
                        if seen.insert(dedupe_key) {
                            hits.push(hit);
                        }
                    }
                }
                continue;
            }

            let Some(lang) = ast_search_lang(normalized_path_ref) else {
                continue;
            };

            if let Ok(content) = std::fs::read_to_string(entry.path()) {
                let crate_name = infer_crate_name(normalized_path_ref);

                for pattern in get_skeleton_patterns(lang) {
                    for result in extract_items(content.as_str(), pattern, lang, Some(vec!["NAME"]))
                    {
                        let name = result.captures.get("NAME").cloned().unwrap_or_else(|| {
                            first_signature_line(result.text.as_str()).to_string()
                        });
                        let signature = first_signature_line(result.text.as_str()).to_string();
                        if signature.is_empty() {
                            continue;
                        }
                        let dedupe_key = format!(
                            "{normalized_path}:{}:{}:{name}",
                            result.line_start, result.line_end
                        );
                        if !seen.insert(dedupe_key) {
                            continue;
                        }

                        hits.push(AstSearchHit {
                            name,
                            signature,
                            path: normalized_path.clone(),
                            language: lang.as_str().to_string(),
                            crate_name: crate_name.clone(),
                            project_name: None,
                            root_label: None,
                            node_kind: None,
                            owner_title: None,
                            navigation_target: ast_navigation_target(
                                normalized_path.as_str(),
                                crate_name.as_str(),
                                None,
                                None,
                                result.line_start,
                                result.line_end,
                            ),
                            line_start: result.line_start,
                            line_end: result.line_end,
                            score: 0.0,
                        });
                    }
                }
            }
        }
    }

    hits
}

fn ast_search_lang(path: &Path) -> Option<Lang> {
    match Lang::from_path(path)? {
        Lang::Python
        | Lang::Rust
        | Lang::JavaScript
        | Lang::TypeScript
        | Lang::Bash
        | Lang::Go
        | Lang::Java
        | Lang::C
        | Lang::Cpp
        | Lang::CSharp
        | Lang::Ruby
        | Lang::Swift
        | Lang::Kotlin
        | Lang::Lua
        | Lang::Php => Lang::from_path(path),
        _ => None,
    }
}
