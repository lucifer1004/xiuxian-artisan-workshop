use super::attachments::attachments_for_parsed_note;
use super::constants::DEFAULT_EXCLUDED_DIR_NAMES;
use super::filters::{merge_excluded_dirs, normalize_include_dir, should_skip_entry};
use super::graphmem::sync_graphmem_state_best_effort;
use crate::link_graph::index::{IndexedSection, LinkGraphIndex, doc_sort_key};
use crate::link_graph::models::{LinkGraphAttachment, LinkGraphDocument};
use crate::link_graph::parser::{ParsedNote, is_supported_note, normalize_alias, parse_note};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

struct NormalizedDirectoryFilters {
    include_dirs: Vec<String>,
    excluded_dirs: Vec<String>,
    included: HashSet<String>,
    excluded: HashSet<String>,
}

struct ParsedNoteMaps {
    docs_by_id: HashMap<String, LinkGraphDocument>,
    sections_by_doc: HashMap<String, Vec<IndexedSection>>,
    attachments_by_doc: HashMap<String, Vec<LinkGraphAttachment>>,
    alias_to_doc_id: HashMap<String, String>,
}

struct GraphEdges {
    outgoing: HashMap<String, HashSet<String>>,
    incoming: HashMap<String, HashSet<String>>,
    edge_count: usize,
}

fn canonicalize_root_dir(root_dir: &Path) -> Result<PathBuf, String> {
    let root = root_dir
        .canonicalize()
        .map_err(|e| format!("invalid notebook root '{}': {e}", root_dir.display()))?;
    if !root.is_dir() {
        return Err(format!(
            "notebook root is not a directory: {}",
            root.display()
        ));
    }
    Ok(root)
}

fn normalize_directory_filters(
    include_dirs: &[String],
    excluded_dirs: &[String],
) -> NormalizedDirectoryFilters {
    let normalized_include_dirs: Vec<String> = include_dirs
        .iter()
        .filter_map(|path| normalize_include_dir(path))
        .collect();
    let normalized_excluded_dirs: Vec<String> =
        merge_excluded_dirs(excluded_dirs, DEFAULT_EXCLUDED_DIR_NAMES);
    let included: HashSet<String> = normalized_include_dirs.iter().cloned().collect();
    let excluded: HashSet<String> = normalized_excluded_dirs.iter().cloned().collect();

    NormalizedDirectoryFilters {
        include_dirs: normalized_include_dirs,
        excluded_dirs: normalized_excluded_dirs,
        included,
        excluded,
    }
}

fn collect_candidate_note_paths(
    root: &Path,
    included: &HashSet<String>,
    excluded: &HashSet<String>,
) -> Vec<PathBuf> {
    let mut candidate_paths: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            !should_skip_entry(
                entry.path(),
                entry.file_type().is_dir(),
                root,
                included,
                excluded,
            )
        })
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !entry.file_type().is_file() || !is_supported_note(path) {
            continue;
        }
        candidate_paths.push(path.to_path_buf());
    }
    candidate_paths
}

fn parse_candidate_notes(root: &Path, candidate_paths: Vec<PathBuf>) -> Vec<ParsedNote> {
    candidate_paths
        .into_par_iter()
        .filter_map(|path| {
            let content = std::fs::read_to_string(&path).ok()?;
            parse_note(&path, root, &content)
        })
        .collect()
}

fn build_note_maps(parsed_notes: &[ParsedNote]) -> ParsedNoteMaps {
    let mut docs_by_id: HashMap<String, LinkGraphDocument> = HashMap::new();
    let mut sections_by_doc: HashMap<String, Vec<IndexedSection>> = HashMap::new();
    let mut attachments_by_doc: HashMap<String, Vec<LinkGraphAttachment>> = HashMap::new();
    let mut alias_to_doc_id: HashMap<String, String> = HashMap::new();

    for parsed in parsed_notes {
        let doc = &parsed.doc;
        docs_by_id.insert(doc.id.clone(), doc.clone());
        let indexed_sections = parsed
            .sections
            .iter()
            .map(IndexedSection::from_parsed)
            .collect::<Vec<IndexedSection>>();
        sections_by_doc.insert(doc.id.clone(), indexed_sections);
        attachments_by_doc.insert(doc.id.clone(), attachments_for_parsed_note(parsed));

        for alias in [&doc.id, &doc.path, &doc.stem] {
            let key = normalize_alias(alias);
            if key.is_empty() {
                continue;
            }
            alias_to_doc_id.entry(key).or_insert_with(|| doc.id.clone());
        }
    }

    ParsedNoteMaps {
        docs_by_id,
        sections_by_doc,
        attachments_by_doc,
        alias_to_doc_id,
    }
}

fn build_graph_edges(
    parsed_notes: Vec<ParsedNote>,
    alias_to_doc_id: &HashMap<String, String>,
) -> GraphEdges {
    let mut outgoing: HashMap<String, HashSet<String>> = HashMap::new();
    let mut incoming: HashMap<String, HashSet<String>> = HashMap::new();
    let mut edge_count = 0usize;

    for parsed in parsed_notes {
        let from_id = parsed.doc.id;
        for raw_target in parsed.link_targets {
            let normalized = normalize_alias(&raw_target);
            if normalized.is_empty() {
                continue;
            }
            let Some(to_id) = alias_to_doc_id.get(&normalized).cloned() else {
                continue;
            };
            if to_id == from_id {
                continue;
            }
            let inserted = outgoing
                .entry(from_id.clone())
                .or_default()
                .insert(to_id.clone());
            if inserted {
                incoming.entry(to_id).or_default().insert(from_id.clone());
                edge_count += 1;
            }
        }
    }

    GraphEdges {
        outgoing,
        incoming,
        edge_count,
    }
}

fn build_index_from_parts(
    root: PathBuf,
    filters: NormalizedDirectoryFilters,
    note_maps: ParsedNoteMaps,
    edges: GraphEdges,
) -> LinkGraphIndex {
    let rank_by_id =
        LinkGraphIndex::compute_rank_by_id(&note_maps.docs_by_id, &edges.incoming, &edges.outgoing);
    let mut index = LinkGraphIndex {
        root,
        include_dirs: filters.include_dirs,
        excluded_dirs: filters.excluded_dirs,
        docs_by_id: note_maps.docs_by_id,
        passages_by_id: HashMap::new(),
        sections_by_doc: note_maps.sections_by_doc,
        attachments_by_doc: note_maps.attachments_by_doc,
        alias_to_doc_id: note_maps.alias_to_doc_id,
        outgoing: edges.outgoing,
        incoming: edges.incoming,
        rank_by_id,
        edge_count: edges.edge_count,
    };
    index.rebuild_all_passages();
    index
}

impl LinkGraphIndex {
    /// Build index with excluded directory names (e.g. ".cache", ".git").
    ///
    /// # Errors
    ///
    /// Returns an error when index construction fails.
    pub fn build_with_excluded_dirs(
        root_dir: &Path,
        excluded_dirs: &[String],
    ) -> Result<Self, String> {
        let index = Self::build_with_filters(root_dir, &[], excluded_dirs)?;
        sync_graphmem_state_best_effort(&index);
        Ok(index)
    }

    /// Build index with include/exclude directory filters relative to notebook root.
    ///
    /// # Errors
    ///
    /// Returns an error when root path validation fails.
    pub fn build_with_filters(
        root_dir: &Path,
        include_dirs: &[String],
        excluded_dirs: &[String],
    ) -> Result<Self, String> {
        let root = canonicalize_root_dir(root_dir)?;
        let filters = normalize_directory_filters(include_dirs, excluded_dirs);
        let candidate_paths =
            collect_candidate_note_paths(&root, &filters.included, &filters.excluded);
        let mut parsed_notes = parse_candidate_notes(&root, candidate_paths);

        parsed_notes.sort_by(|left, right| doc_sort_key(&left.doc).cmp(&doc_sort_key(&right.doc)));

        let note_maps = build_note_maps(&parsed_notes);
        let graph_edges = build_graph_edges(parsed_notes, &note_maps.alias_to_doc_id);
        Ok(build_index_from_parts(
            root,
            filters,
            note_maps,
            graph_edges,
        ))
    }
}
