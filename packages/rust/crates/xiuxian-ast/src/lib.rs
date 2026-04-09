//! xiuxian-ast - Unified AST Utilities using ast-grep
//!
//! This crate provides a unified interface for AST-based code analysis
//! across the Omni `DevEnv` project.
//!
//! ## Architecture
//!
//! ```text
//! xiuxian-ast/src/
//! ├── lib.rs                # Re-exports (entry point)
//! ├── re_exports.rs         # ast-grep re-exports
//! ├── lang.rs               # Lang enum and language support
//! ├── match.rs              # Match struct
//! ├── scan.rs               # Pattern utilities
//! ├── python.rs             # Python-specific utilities
//! ├── python_tree_sitter.rs # Tree-sitter Python parser
//! ├── julia_tree_sitter.rs  # Tree-sitter Julia parser (feature: julia)
//! └── modelica_tree_sitter.rs # Tree-sitter Modelica parser (feature: modelica)
//! ```
//!
//! ## Quick Start
//!
//! ```rust
//! use xiuxian_ast::{scan, Lang};
//!
//! let content = "def hello(): pass";
//! let matches = scan(content, "def $NAME", Lang::Python)
//!     .unwrap_or_else(|error| panic!("scan failed: {error}"));
//! ```
//!
//! ## Optional Features
//!
//! - `julia`: Enable Julia tree-sitter parser
//! - `modelica`: Enable Modelica tree-sitter parser (runtime-loaded)

xiuxian_testing::crate_test_policy_source_harness!("../tests/unit/lib_policy.rs");

// ============================================================================
// Module Declarations
// ============================================================================

mod chunk;
mod extract;
mod item;
#[cfg(feature = "julia")]
mod julia_tree_sitter;
mod lang;
#[cfg(feature = "modelica")]
mod modelica_tree_sitter;
mod python;
mod python_tree_sitter;
mod re_exports;
mod scan;
mod security;

// ============================================================================
// Re-exports (for backwards compatibility)
// ============================================================================

// Re-exports module
pub use re_exports::*;

// Lang enum
pub use lang::Lang;

// Match struct
pub use item::Match;

// Scan functions (both direct and re-exported from scan module)
pub use scan::{extract, pattern, scan, scan_with_lang};

// Python utilities
pub use python::{
    extract_docstring_from_match, extract_python_docstring, find_python_async_functions,
    find_python_classes, find_python_decorated_by_any, find_python_decorated_functions,
    find_python_functions,
};

// Security scanner for harvested skills
pub use security::{SecurityConfig, SecurityScanner, SecurityViolation};

// Code extraction utilities
pub use extract::{ExtractResult, extract_items, extract_skeleton, get_skeleton_patterns};

// Code chunking utilities
pub use chunk::{CodeChunk, chunk_code};

// Tree-sitter based Python parser for robust decorator extraction
pub use python_tree_sitter::{
    DecoratedFunction, DecoratorArguments, DecoratorInfo, ParameterInfo, TreeSitterPythonParser,
};

#[cfg(feature = "julia")]
pub use julia_tree_sitter::{
    JuliaDocAttachment, JuliaDocTargetKind, JuliaFileSummary, JuliaImport, JuliaParseError,
    JuliaSourceSummary, JuliaSymbol, JuliaSymbolKind, TreeSitterJuliaParser,
};

#[cfg(feature = "modelica")]
pub use modelica_tree_sitter::{
    ModelicaComponent, ModelicaComponentKind, ModelicaFileSummary, ModelicaImport,
    ModelicaParseError, ModelicaSymbol, ModelicaSymbolKind, ModelicaVisibility,
    TreeSitterModelicaParser,
};
