//! Tree-sitter based Modelica parser for conservative repository-entry extraction.
//!
//! This module loads the tree-sitter-modelica grammar at runtime from a shared library.

#![allow(unsafe_code)]

use std::env::consts::OS;
use std::path::PathBuf;

use libloading::{Library, Symbol};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tree_sitter::{Language, Node, Parser};

/// Errors raised while loading or parsing Modelica source.
#[derive(Debug, Error)]
pub enum ModelicaParseError {
    /// The tree-sitter-modelica library could not be found.
    #[error("failed to find tree-sitter-modelica library: {0}")]
    LibraryNotFound(String),
    /// The tree-sitter-modelica library could not be loaded.
    #[error("failed to load tree-sitter-modelica library: {0}")]
    LibraryLoad(String),
    /// The language symbol could not be found in the library.
    #[error("failed to find tree_sitter_modelica symbol in library")]
    SymbolNotFound,
    /// Tree-sitter did not produce a syntax tree for the provided source.
    #[error("failed to parse Modelica source")]
    ParseFailed,
}

/// Supported Modelica symbol kinds for conservative AST extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelicaSymbolKind {
    /// A class-like declaration (model, block, record, etc.).
    Class,
    /// A function declaration.
    Function,
    /// A package declaration.
    Package,
    /// A connector declaration.
    Connector,
    /// A type declaration.
    Type,
    /// A constant or parameter.
    Constant,
}

/// Visibility modifier for Modelica declarations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelicaVisibility {
    /// Public visibility (default).
    #[default]
    Public,
    /// Protected visibility.
    Protected,
    /// Encapsulated (no external lookups).
    Encapsulated,
    /// Partial (incomplete definition).
    Partial,
}

/// Component kind for Modelica variables and parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelicaComponentKind {
    /// A parameter value.
    Parameter,
    /// A constant value.
    Constant,
    /// A regular variable.
    Variable,
    /// An input connector.
    InputConnector,
    /// An output connector.
    OutputConnector,
    /// An inner element.
    Inner,
    /// An outer element reference.
    Outer,
}

/// One Modelica symbol extracted from source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelicaSymbol {
    /// Symbol display name.
    pub name: String,
    /// Normalized symbol kind.
    pub kind: ModelicaSymbolKind,
    /// Optional signature snippet.
    pub signature: Option<String>,
    /// Source location: starting line number (1-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<usize>,
    /// Source location: ending line number (1-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_end: Option<usize>,
    /// Visibility modifier.
    #[serde(default, skip_serializing_if = "ModelicaVisibility::is_default_ref")]
    pub visibility: ModelicaVisibility,
    /// Components (parameters, variables) within this symbol.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<ModelicaComponent>,
    /// Mathematical equations within this symbol.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub equations: Vec<String>,
}

/// One Modelica component (parameter, variable, connector) extracted from source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelicaComponent {
    /// Component display name.
    pub name: String,
    /// Type name (e.g., "Real", "Modelica.SIunits.Length").
    pub type_name: String,
    /// Component kind.
    pub kind: ModelicaComponentKind,
    /// Default value or modification.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    /// Unit annotation if present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Source location: starting line number (1-based).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<usize>,
}

/// One Modelica import-like relation extracted from source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ModelicaImport {
    /// Imported package/class name.
    pub name: String,
    /// Import kind (e.g., full path vs. short name).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

/// Conservative Modelica source summary for repository-entry analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelicaFileSummary {
    /// Optional package/class name declared in the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    /// Imported or used packages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<ModelicaImport>,
    /// Directly declared classes, functions, etc.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub symbols: Vec<ModelicaSymbol>,
    /// Literal `annotation(Documentation(...))` content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
}

impl ModelicaVisibility {
    /// Check if this is the default visibility.
    #[must_use]
    pub const fn is_default(self) -> bool {
        matches!(self, Self::Public)
    }

    /// Check if a referenced visibility is the default value.
    #[must_use]
    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub const fn is_default_ref(value: &Self) -> bool {
        value.is_default()
    }
}

/// Returns the expected library filename for the current platform.
fn library_name() -> &'static str {
    if OS == "macos" {
        "libtree-sitter-modelica.dylib"
    } else if OS == "windows" {
        "tree-sitter-modelica.dll"
    } else {
        "libtree-sitter-modelica.so"
    }
}

/// Finds the tree-sitter-modelica library path.
fn find_library_path() -> Result<PathBuf, ModelicaParseError> {
    if let Ok(path) = std::env::var("XIUXIAN_TREE_SITTER_MODELICA_LIB") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
    }

    let lib_name = library_name();
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").map_or_else(|_| PathBuf::from("."), PathBuf::from);

    let resource_path = manifest_dir.join("resources").join(lib_name);
    if resource_path.exists() {
        return Ok(resource_path);
    }

    let cwd_path = PathBuf::from(lib_name);
    if cwd_path.exists() {
        return Ok(cwd_path);
    }

    Err(ModelicaParseError::LibraryNotFound(format!(
        "Could not find {lib_name}."
    )))
}

/// Tree-sitter based Modelica parser with runtime-loaded grammar.
pub struct TreeSitterModelicaParser {
    parser: Parser,
    #[allow(dead_code)]
    _library: Library,
}

impl TreeSitterModelicaParser {
    /// Create a runtime-loaded Modelica parser backed by tree-sitter.
    ///
    /// # Errors
    ///
    /// Returns an error when the Modelica grammar shared library cannot be
    /// found, loaded, or initialized.
    pub fn new() -> Result<Self, ModelicaParseError> {
        let lib_path = find_library_path()?;

        unsafe {
            let library = Library::new(&lib_path).map_err(|e| {
                ModelicaParseError::LibraryLoad(format!("{}: {}", lib_path.display(), e))
            })?;

            let tree_sitter_modelica: Symbol<unsafe extern "C" fn() -> Language> = library
                .get(b"tree_sitter_modelica")
                .map_err(|_| ModelicaParseError::SymbolNotFound)?;

            let language = tree_sitter_modelica();
            let mut parser = Parser::new();
            parser
                .set_language(&language)
                .map_err(|e| ModelicaParseError::LibraryLoad(e.to_string()))?;

            Ok(Self {
                parser,
                _library: library,
            })
        }
    }

    /// Parse one Modelica source file into a conservative summary.
    ///
    /// # Errors
    ///
    /// Returns an error when tree-sitter cannot parse the provided source or
    /// the runtime grammar cannot produce a syntax tree.
    pub fn parse_file_summary(
        &mut self,
        code: &str,
    ) -> Result<ModelicaFileSummary, ModelicaParseError> {
        let tree = self
            .parser
            .parse(code, None)
            .ok_or(ModelicaParseError::ParseFailed)?;

        let mut class_name = None;
        let mut imports = Vec::new();
        let mut symbols = Vec::new();

        collect_summary(
            tree.root_node(),
            code,
            &mut class_name,
            &mut imports,
            &mut symbols,
            0,
        );

        Ok(ModelicaFileSummary {
            class_name,
            imports,
            symbols,
            documentation: extract_documentation(code),
        })
    }
}

fn collect_summary(
    node: Node<'_>,
    code: &str,
    class_name: &mut Option<String>,
    imports: &mut Vec<ModelicaImport>,
    symbols: &mut Vec<ModelicaSymbol>,
    depth: usize,
) {
    if node.kind() == "class_definition" {
        if class_name.is_none() && depth < 4 {
            *class_name = extract_class_name_from_definition(node, code);
        }
        if depth >= 3
            && let Some(symbol) = extract_symbol_from_definition(node, code)
        {
            symbols.push(symbol);
        }
    } else if node.kind() == "import_clause"
        && let Some(import) = extract_import_from_clause(node, code)
    {
        imports.push(import);
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_summary(child, code, class_name, imports, symbols, depth + 1);
    }
}

fn extract_class_name_from_definition(node: Node<'_>, code: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if matches!(
            child.kind(),
            "long_class_specifier" | "short_class_specifier"
        ) {
            let mut inner_cursor = child.walk();
            for inner_child in child.named_children(&mut inner_cursor) {
                if inner_child.kind() == "IDENT" {
                    return inner_child
                        .utf8_text(code.as_bytes())
                        .ok()
                        .map(str::to_string);
                }
            }
        }
    }
    None
}

fn extract_class_prefix(node: Node<'_>, code: &str) -> Option<String> {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "class_prefixes" {
            return child
                .utf8_text(code.as_bytes())
                .ok()
                .map(|s| s.trim().to_string());
        }
    }
    None
}

fn extract_symbol_from_definition(node: Node<'_>, code: &str) -> Option<ModelicaSymbol> {
    let prefix = extract_class_prefix(node, code)?;
    let visibility = extract_visibility_from_prefix(&prefix);
    let kind = match prefix.as_str() {
        "package" => ModelicaSymbolKind::Package,
        "connector" | "expandable connector" => ModelicaSymbolKind::Connector,
        "type" => ModelicaSymbolKind::Type,
        "function" => ModelicaSymbolKind::Function,
        "constant" | "parameter" => ModelicaSymbolKind::Constant,
        _ => ModelicaSymbolKind::Class,
    };

    let name = extract_class_name_from_definition(node, code)?;
    let signature = node
        .utf8_text(code.as_bytes())
        .ok()
        .map(|s| s.lines().next().unwrap_or("").trim().to_string());

    let components = extract_components_from_class(node, code);
    let equations = extract_equations_from_class(node, code);

    Some(ModelicaSymbol {
        name,
        kind,
        signature,
        line_start: Some(node.start_position().row + 1),
        line_end: Some(node.end_position().row + 1),
        visibility,
        components,
        equations,
    })
}

fn extract_visibility_from_prefix(prefix: &str) -> ModelicaVisibility {
    let lower = prefix.to_ascii_lowercase();
    if lower.contains("encapsulated") {
        ModelicaVisibility::Encapsulated
    } else if lower.contains("partial") {
        ModelicaVisibility::Partial
    } else if lower.contains("protected") {
        ModelicaVisibility::Protected
    } else {
        ModelicaVisibility::Public
    }
}

fn extract_components_from_class(node: Node<'_>, code: &str) -> Vec<ModelicaComponent> {
    let mut components = Vec::new();
    collect_components(node, code, &mut components);
    components
}

fn collect_components(node: Node<'_>, code: &str, components: &mut Vec<ModelicaComponent>) {
    if node.kind() == "component_clause" {
        if let Some(component) = extract_component_from_clause(node, code) {
            components.push(component);
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_components(child, code, components);
    }
}

fn extract_component_from_clause(node: Node<'_>, code: &str) -> Option<ModelicaComponent> {
    let mut cursor = node.walk();
    let mut kind = ModelicaComponentKind::Variable;
    let mut type_name = String::new();
    let mut name = String::new();
    let mut default_value = None;
    let mut unit = None;

    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "type_prefix" => {
                let prefix = child.utf8_text(code.as_bytes()).ok()?;
                kind = match prefix.trim().to_ascii_lowercase().as_str() {
                    "parameter" => ModelicaComponentKind::Parameter,
                    "constant" => ModelicaComponentKind::Constant,
                    "input" => ModelicaComponentKind::InputConnector,
                    "output" => ModelicaComponentKind::OutputConnector,
                    "inner" => ModelicaComponentKind::Inner,
                    "outer" => ModelicaComponentKind::Outer,
                    "flow" | "stream" => ModelicaComponentKind::Variable,
                    _ => kind,
                };
            }
            "type_specifier" => {
                type_name = child.utf8_text(code.as_bytes()).ok()?.trim().to_string();
            }
            "declaration" => {
                let mut decl_cursor = child.walk();
                for decl_child in child.named_children(&mut decl_cursor) {
                    match decl_child.kind() {
                        "IDENT" => {
                            name = decl_child.utf8_text(code.as_bytes()).ok()?.to_string();
                        }
                        "modification" => {
                            default_value = decl_child
                                .utf8_text(code.as_bytes())
                                .ok()
                                .map(|s| s.trim().to_string());
                            if let Some(unit_val) = extract_unit_from_modification(decl_child, code)
                            {
                                unit = Some(unit_val);
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    if name.is_empty() || type_name.is_empty() {
        return None;
    }

    Some(ModelicaComponent {
        name,
        type_name,
        kind,
        default_value,
        unit,
        line_start: Some(node.start_position().row + 1),
    })
}

fn extract_unit_from_modification(node: Node<'_>, code: &str) -> Option<String> {
    let text = node.utf8_text(code.as_bytes()).ok()?;
    let unit_start = text.find("unit=\"")? + 6;
    let remaining = &text[unit_start..];
    let unit_end = remaining.find('"')?;
    Some(remaining[..unit_end].to_string())
}

fn extract_equations_from_class(node: Node<'_>, code: &str) -> Vec<String> {
    let mut equations = Vec::new();
    collect_equations(node, code, &mut equations);
    equations
}

fn collect_equations(node: Node<'_>, code: &str, equations: &mut Vec<String>) {
    if node.kind() == "equation_section" {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            if child.kind() == "equation"
                && let Ok(text) = child.utf8_text(code.as_bytes())
            {
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    equations.push(trimmed);
                }
            }
        }
        return;
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_equations(child, code, equations);
    }
}

fn extract_import_from_clause(node: Node<'_>, code: &str) -> Option<ModelicaImport> {
    let mut cursor = node.walk();
    let mut idents = Vec::new();
    let mut name_node = None;

    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "IDENT" => idents.push(child.utf8_text(code.as_bytes()).ok()?.to_string()),
            "name" => name_node = Some(child),
            _ => {}
        }
    }

    if let Some(name_node) = name_node {
        let full_name = extract_full_name(&name_node, code);
        let alias = idents.first().cloned();
        Some(ModelicaImport {
            name: full_name,
            alias,
        })
    } else if !idents.is_empty() {
        Some(ModelicaImport {
            name: idents.join("."),
            alias: None,
        })
    } else {
        None
    }
}

fn extract_full_name(node: &Node<'_>, code: &str) -> String {
    let mut parts = Vec::new();
    collect_name_parts(*node, code, &mut parts);
    parts.join(".")
}

fn collect_name_parts(node: Node<'_>, code: &str, parts: &mut Vec<String>) {
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        match child.kind() {
            "IDENT" => {
                if let Ok(text) = child.utf8_text(code.as_bytes()) {
                    parts.push(text.to_string());
                }
            }
            "name" => {
                collect_name_parts(child, code, parts);
            }
            _ => {}
        }
    }
}

fn extract_documentation(code: &str) -> Option<String> {
    let start = code.find("annotation(")?;
    let doc_start = code[start..].find("Documentation(")? + start;
    let content_start = doc_start + "Documentation(".len();

    let mut depth = 1;
    let mut end = content_start;
    for (i, c) in code[content_start..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = content_start + i;
                    break;
                }
            }
            _ => {}
        }
    }

    Some(code[content_start..end].to_string())
}

#[cfg(test)]
#[path = "../tests/unit/modelica_tree_sitter.rs"]
mod tests;
