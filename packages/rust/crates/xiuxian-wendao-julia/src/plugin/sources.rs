use std::collections::{BTreeSet, VecDeque};
use std::fs;
use std::path::Path;

use xiuxian_ast::{JuliaFileSummary, JuliaSourceSummary, TreeSitterJuliaParser};

use xiuxian_wendao_core::repo_intelligence::{DiagnosticRecord, RepoIntelligenceError};

use super::discovery::relative_path_string;

#[derive(Debug, Clone)]
pub(crate) struct JuliaAnalyzedFile {
    pub(crate) path: String,
    pub(crate) summary: JuliaFileSummary,
}

#[derive(Debug, Clone)]
pub(crate) struct JuliaCollectedSources {
    pub(crate) root_summary: JuliaSourceSummary,
    pub(crate) files: Vec<JuliaAnalyzedFile>,
}

pub(crate) fn collect_julia_sources(
    repo_id: &str,
    repository_root: &Path,
    root_file: &Path,
    diagnostics: &mut Vec<DiagnosticRecord>,
) -> Result<JuliaCollectedSources, RepoIntelligenceError> {
    let root_path = relative_path_string(repository_root, root_file)?;
    let root_contents =
        fs::read_to_string(root_file).map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to read Julia root file `{}`: {error}",
                root_file.display()
            ),
        })?;

    let mut parser = TreeSitterJuliaParser::new().map_err(map_parse_error(root_path.clone()))?;
    let root_summary = parser
        .parse_summary(&root_contents)
        .map_err(map_parse_error(root_path.clone()))?;
    let root_file_summary = JuliaFileSummary {
        module_name: Some(root_summary.module_name.clone()),
        exports: root_summary.exports.clone(),
        imports: root_summary.imports.clone(),
        symbols: root_summary.symbols.clone(),
        docstrings: root_summary.docstrings.clone(),
        includes: root_summary.includes.clone(),
    };

    let mut files = vec![JuliaAnalyzedFile {
        path: root_path.clone(),
        summary: root_file_summary.clone(),
    }];
    let mut visited = BTreeSet::from([root_path]);
    let mut pending = root_file_summary
        .includes
        .iter()
        .map(|include| (root_file.to_path_buf(), include.clone()))
        .collect::<VecDeque<_>>();

    while let Some((including_file, include_literal)) = pending.pop_front() {
        let include_path = including_file
            .parent()
            .unwrap_or(repository_root)
            .join(include_literal.as_str());
        let Ok(include_relative) = relative_path_string(repository_root, &include_path) else {
            diagnostics.push(DiagnosticRecord {
                repo_id: repo_id.to_string(),
                path: include_literal.clone(),
                line: 0,
                message: format!(
                    "ignored include `{include_literal}` because it resolves outside repository root"
                ),
                severity: "warning".to_string(),
            });
            continue;
        };

        if !include_path.is_file() {
            diagnostics.push(DiagnosticRecord {
                repo_id: repo_id.to_string(),
                path: include_relative,
                line: 0,
                message: format!(
                    "ignored include `{include_literal}` because the target file does not exist"
                ),
                severity: "warning".to_string(),
            });
            continue;
        }
        if !visited.insert(include_relative.clone()) {
            continue;
        }

        let include_contents = fs::read_to_string(&include_path).map_err(|error| {
            RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to read included Julia file `{}`: {error}",
                    include_path.display()
                ),
            }
        })?;
        let summary = parser
            .parse_file_summary(&include_contents)
            .map_err(map_parse_error(include_relative.clone()))?;
        pending.extend(
            summary
                .includes
                .iter()
                .map(|include| (include_path.clone(), include.clone())),
        );
        files.push(JuliaAnalyzedFile {
            path: include_relative,
            summary,
        });
    }

    Ok(JuliaCollectedSources {
        root_summary,
        files,
    })
}

fn map_parse_error(
    path: String,
) -> impl FnOnce(xiuxian_ast::JuliaParseError) -> RepoIntelligenceError {
    move |error| RepoIntelligenceError::AnalysisFailed {
        message: format!("failed to parse Julia source `{path}`: {error}"),
    }
}
