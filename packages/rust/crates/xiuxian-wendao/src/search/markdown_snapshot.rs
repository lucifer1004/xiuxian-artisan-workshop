use std::collections::BTreeMap;
use std::path::Path;

use crate::gateway::studio::compile_markdown_nodes;
use crate::gateway::studio::search::source_index::{
    build_markdown_ast_hits_from_sections, markdown_scope_name,
};
use crate::gateway::studio::types::AstSearchHit;
use crate::parsers::markdown::{ParsedNote, adapt_markdown_note, is_supported_note};
use xiuxian_wendao_parsers::parse_markdown_note;

use super::ProjectScannedFile;

#[derive(Debug, Clone, Default)]
pub(crate) struct MarkdownProjectSnapshot {
    entries_by_path: BTreeMap<String, std::sync::Arc<MarkdownSnapshotEntry>>,
}

impl MarkdownProjectSnapshot {
    #[must_use]
    pub(crate) fn new(
        entries_by_path: BTreeMap<String, std::sync::Arc<MarkdownSnapshotEntry>>,
    ) -> Self {
        Self { entries_by_path }
    }

    #[must_use]
    pub(crate) fn entry(
        &self,
        normalized_path: &str,
    ) -> Option<&std::sync::Arc<MarkdownSnapshotEntry>> {
        self.entries_by_path.get(normalized_path)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct MarkdownSnapshotEntry {
    pub(crate) file: ProjectScannedFile,
    pub(crate) parsed_note: Option<ParsedNote>,
    pub(crate) ast_hits: Vec<AstSearchHit>,
}

#[must_use]
pub(crate) fn markdown_snapshot_entry_cache_key(
    project_root: &Path,
    file: &ProjectScannedFile,
) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(project_root.to_string_lossy().as_bytes());
    hasher.update(file.scan_root.to_string_lossy().as_bytes());
    hasher.update(file.partition_id.as_bytes());
    hasher.update(file.absolute_path.to_string_lossy().as_bytes());
    hasher.update(file.normalized_path.as_bytes());
    hasher.update(file.project_name.as_deref().unwrap_or_default().as_bytes());
    hasher.update(file.root_label.as_deref().unwrap_or_default().as_bytes());
    hasher.update(&file.size_bytes.to_le_bytes());
    hasher.update(&file.modified_secs.to_le_bytes());
    hasher.update(&u64::from(file.modified_nanos).to_le_bytes());
    hasher.finalize().to_hex().to_string()
}

#[must_use]
pub(crate) fn build_markdown_snapshot_entry(
    project_root: &Path,
    file: &ProjectScannedFile,
) -> MarkdownSnapshotEntry {
    if !is_supported_note(file.absolute_path.as_path()) {
        return MarkdownSnapshotEntry {
            file: file.clone(),
            parsed_note: None,
            ast_hits: Vec::new(),
        };
    }

    let Ok(content) = std::fs::read_to_string(file.absolute_path.as_path()) else {
        return MarkdownSnapshotEntry {
            file: file.clone(),
            parsed_note: None,
            ast_hits: Vec::new(),
        };
    };

    let fallback_title = file
        .absolute_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("page");
    let parser_note = parse_markdown_note(&content, fallback_title);
    let nodes = compile_markdown_nodes(file.normalized_path.as_str(), &content);
    let crate_name = markdown_scope_name(Path::new(file.normalized_path.as_str()));
    let mut ast_hits = build_markdown_ast_hits_from_sections(
        file.normalized_path.as_str(),
        crate_name.as_str(),
        &nodes,
        parser_note.core.sections.as_slice(),
    );
    let parsed_note = adapt_markdown_note(file.absolute_path.as_path(), project_root, parser_note);
    for hit in &mut ast_hits {
        if file.project_name.is_some() {
            hit.project_name.clone_from(&file.project_name);
            hit.navigation_target
                .project_name
                .clone_from(&file.project_name);
        }
        if file.root_label.is_some() {
            hit.root_label.clone_from(&file.root_label);
            hit.navigation_target
                .root_label
                .clone_from(&file.root_label);
        }
    }

    MarkdownSnapshotEntry {
        file: file.clone(),
        parsed_note,
        ast_hits,
    }
}
