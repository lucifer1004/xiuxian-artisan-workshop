use std::collections::HashMap;

use super::{UnifiedSymbol, symbol::SymbolSource};
use crate::search::SearchDocumentIndex;

mod add;
mod query;
mod stats;

/// Unified Symbol Index - combines project and external symbols.
pub struct UnifiedSymbolIndex {
    /// All symbols indexed by lowercase name (Legacy in-memory path)
    pub(crate) by_name: HashMap<String, Vec<usize>>,
    /// All symbols stored in a vector
    pub(crate) symbols: Vec<UnifiedSymbol>,
    /// External crate usage in project (`crate_name` -> project locations)
    pub(crate) external_usage: HashMap<String, Vec<String>>,
    /// Project files that use external crates
    pub(crate) project_files: HashMap<String, Vec<String>>,

    // Shared search integration
    pub(crate) search_index: SearchDocumentIndex,
}

impl UnifiedSymbolIndex {
    /// Create an empty unified index.
    #[must_use]
    pub fn new() -> Self {
        Self {
            by_name: HashMap::new(),
            symbols: Vec::new(),
            external_usage: HashMap::new(),
            project_files: HashMap::new(),
            search_index: SearchDocumentIndex::new(),
        }
    }

    /// Returns list of all unique external crate names in the index.
    pub fn get_external_crates(&self) -> Vec<String> {
        let mut crates = Vec::new();
        for symbol in &self.symbols {
            if let SymbolSource::External(ref name) = symbol.source {
                crates.push(name.clone());
            }
        }
        crates.sort();
        crates.dedup();
        crates
    }

    /// Returns list of all unique project-local crate names in the index.
    pub fn get_project_crates(&self) -> Vec<String> {
        let mut crates = Vec::new();
        for symbol in &self.symbols {
            if symbol.source == SymbolSource::Project {
                crates.push(symbol.crate_name.clone());
            }
        }
        crates.sort();
        crates.dedup();
        crates
    }
}

impl Clone for UnifiedSymbolIndex {
    fn clone(&self) -> Self {
        Self {
            by_name: self.by_name.clone(),
            symbols: self.symbols.clone(),
            external_usage: self.external_usage.clone(),
            project_files: self.project_files.clone(),
            search_index: self.search_index.clone(),
        }
    }
}

impl std::fmt::Debug for UnifiedSymbolIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnifiedSymbolIndex")
            .field("symbol_count", &self.symbols.len())
            .finish()
    }
}
