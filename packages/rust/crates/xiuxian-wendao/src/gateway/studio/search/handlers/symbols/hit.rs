use std::path::Path;

use crate::gateway::studio::search::project_scope::project_metadata_for_path;
use crate::gateway::studio::types::{StudioNavigationTarget, SymbolSearchHit, UiProjectConfig};

pub(super) fn symbol_search_hit(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    symbol: crate::unified_symbol::UnifiedSymbol,
    rank: usize,
) -> SymbolSearchHit {
    let (path, line) = parse_symbol_location(symbol.location.as_str());
    let metadata = project_metadata_for_path(project_root, config_root, projects, path.as_str());
    let source = if symbol.is_project() {
        "project".to_string()
    } else {
        "external".to_string()
    };
    let language =
        crate::gateway::studio::search::support::source_language_label(Path::new(path.as_str()))
            .unwrap_or("unknown")
            .to_string();

    SymbolSearchHit {
        name: symbol.name,
        kind: symbol.kind,
        path: path.clone(),
        line,
        location: symbol.location,
        language,
        source,
        crate_name: symbol.crate_name,
        project_name: metadata.project_name.clone(),
        root_label: metadata.root_label.clone(),
        navigation_target: StudioNavigationTarget {
            path,
            category: "doc".to_string(),
            project_name: metadata.project_name,
            root_label: metadata.root_label,
            line: Some(line),
            line_end: Some(line),
            column: None,
        },
        score: if rank == usize::MAX { 0.0 } else { 0.95 },
    }
}

fn parse_symbol_location(location: &str) -> (String, usize) {
    match location.rsplit_once(':') {
        Some((path, line)) => (path.to_string(), line.parse::<usize>().unwrap_or(1)),
        None => (location.to_string(), 1),
    }
}
