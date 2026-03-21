//! Tree-sitter based Modelica parser for conservative repository-entry extraction.
//!
//! This module loads the tree-sitter-modelica grammar at runtime from a shared library.

#![allow(unsafe_code)]

use std::env::consts::OS;
use std::path::PathBuf;

use libloading::{Library, Symbol};
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// One Modelica symbol extracted from source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelicaSymbol {
    /// Symbol display name.
    pub name: String,
    /// Normalized symbol kind.
    pub kind: ModelicaSymbolKind,
    /// Optional signature snippet.
    pub signature: Option<String>,
}

/// One Modelica import-like relation extracted from source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModelicaImport {
    /// Imported package/class name.
    pub name: String,
    /// Import kind (e.g., full path vs. short name).
    pub alias: Option<String>,
}

/// Conservative Modelica source summary for repository-entry analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelicaFileSummary {
    /// Optional package/class name declared in the file.
    pub class_name: Option<String>,
    /// Imported or used packages.
    pub imports: Vec<ModelicaImport>,
    /// Directly declared classes, functions, etc.
    pub symbols: Vec<ModelicaSymbol>,
    /// Literal `annotation(Documentation(...))` content.
    pub documentation: Option<String>,
}

/// Returns the expected library filename for the current platform.
fn library_name() -> &'static str {
    match OS {
        "macos" => "libtree-sitter-modelica.dylib",
        "linux" => "libtree-sitter-modelica.so",
        "windows" => "tree-sitter-modelica.dll",
        _ => "libtree-sitter-modelica.so",
    }
}

/// Finds the tree-sitter-modelica library path.
///
/// Search order:
/// 1. `XIUXIAN_TREE_SITTER_MODELICA_LIB` environment variable
/// 2. `resources/` directory relative to the crate
/// 3. System library paths (via libloading)
fn find_library_path() -> Result<PathBuf, ModelicaParseError> {
    // 1. Check environment variable
    if let Ok(path) = std::env::var("XIUXIAN_TREE_SITTER_MODELICA_LIB") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
    }

    // 2. Check resources directory relative to crate
    let lib_name = library_name();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));

    let resource_path = manifest_dir.join("resources").join(lib_name);
    if resource_path.exists() {
        return Ok(resource_path);
    }

    // 3. Try current directory
    let cwd_path = PathBuf::from(lib_name);
    if cwd_path.exists() {
        return Ok(cwd_path);
    }

    Err(ModelicaParseError::LibraryNotFound(format!(
        "Could not find {lib_name}. Set XIUXIAN_TREE_SITTER_MODELICA_LIB environment variable or place library in resources/"
    )))
}

/// Tree-sitter based Modelica parser with runtime-loaded grammar.
pub struct TreeSitterModelicaParser {
    parser: Parser,
    #[allow(dead_code)]
    _library: Library, // Keep library loaded for the lifetime of the parser
}

impl TreeSitterModelicaParser {
    /// Create a new Modelica parser by loading the grammar at runtime.
    ///
    /// # Errors
    ///
    /// Returns [`ModelicaParseError`] when the Modelica tree-sitter library
    /// cannot be found or loaded.
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
    /// Returns [`ModelicaParseError`] when parsing fails.
    pub fn parse_file_summary(&mut self, code: &str) -> Result<ModelicaFileSummary, ModelicaParseError> {
        let tree = self
            .parser
            .parse(code, None)
            .ok_or(ModelicaParseError::ParseFailed)?;

        let mut class_name = None;
        let mut imports = Vec::new();
        let mut symbols = Vec::new();

        collect_summary(tree.root_node(), code, &mut class_name, &mut imports, &mut symbols);

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
) {
    let _node_text = || node.utf8_text(code.as_bytes()).unwrap_or("");

    match node.kind() {
        "class_definition" | "model_definition" | "block_definition" | "record_definition"
        | "package_definition" | "connector_definition" | "type_definition" => {
            if class_name.is_none() {
                *class_name = extract_class_name(node, code);
            }
            if let Some(symbol) = extract_symbol(node, code) {
                symbols.push(symbol);
            }
        }
        "function_definition" => {
            if let Some(symbol) = extract_function_symbol(node, code) {
                symbols.push(symbol);
            }
        }
        "import_clause" => {
            if let Some(import) = extract_import(node, code) {
                imports.push(import);
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_summary(child, code, class_name, imports, symbols);
    }
}

fn extract_class_name(node: Node<'_>, code: &str) -> Option<String> {
    // Look for the identifier in the class definition
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        if child.kind() == "identifier" || child.kind() == "name" {
            return Some(child.utf8_text(code.as_bytes()).ok()?.to_string());
        }
    }
    None
}

fn extract_symbol(node: Node<'_>, code: &str) -> Option<ModelicaSymbol> {
    let kind = match node.kind() {
        "class_definition" | "model_definition" => ModelicaSymbolKind::Class,
        "block_definition" | "record_definition" => ModelicaSymbolKind::Class,
        "package_definition" => ModelicaSymbolKind::Package,
        "connector_definition" => ModelicaSymbolKind::Connector,
        "type_definition" => ModelicaSymbolKind::Type,
        _ => return None,
    };

    let name = extract_class_name(node, code)?;
    let signature = node.utf8_text(code.as_bytes()).ok().map(|s| {
        s.lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string()
    });

    Some(ModelicaSymbol {
        name,
        kind,
        signature,
    })
}

fn extract_function_symbol(node: Node<'_>, code: &str) -> Option<ModelicaSymbol> {
    let name = extract_class_name(node, code)?;
    let signature = node.utf8_text(code.as_bytes()).ok().map(|s| {
        s.lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string()
    });

    Some(ModelicaSymbol {
        name,
        kind: ModelicaSymbolKind::Function,
        signature,
    })
}

fn extract_import(node: Node<'_>, code: &str) -> Option<ModelicaImport> {
    let text = node.utf8_text(code.as_bytes()).ok()?;
    // Simple import extraction - could be enhanced with proper AST walking
    let text = text.trim();
    let text = text.trim_start_matches("import ");
    let text = text.trim_end_matches(';');

    let (name, alias) = if let Some((base, alias)) = text.split_once('=') {
        (alias.trim().to_string(), Some(base.trim().to_string()))
    } else {
        (text.to_string(), None)
    };

    Some(ModelicaImport { name, alias })
}

fn extract_documentation(code: &str) -> Option<String> {
    // Look for annotation(Documentation(...)) pattern
    let start = code.find("annotation(")?;
    let doc_start = code[start..].find("Documentation(")? + start;
    let content_start = doc_start + "Documentation(".len();

    // Simple bracket matching to find the end
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
mod tests {
    use super::TreeSitterModelicaParser;

    #[test]
    fn parse_modelica_file_summary() {
        let mut parser = match TreeSitterModelicaParser::new() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Skipping test: {e}");
                return;
            }
        };

        let code = r#"
package MyPackage
  import Modelica.SIunits;
  import M = Modelica.Math;

  model MyModel
    parameter Real x = 1;
    Real y;
  equation
    y = x * 2;
  end MyModel;

  function myFunction
    input Real a;
    output Real b;
  algorithm
    b := a * 2;
  end myFunction;

  annotation(Documentation(info="<html>Package docs</html>"));
end MyPackage;
"#;

        // Debug: print tree structure
        let tree = parser.parser.parse(code, None).unwrap();
        fn print_tree(node: tree_sitter::Node, code: &str, indent: usize) {
            let pad = "  ".repeat(indent);
            let text = node.utf8_text(code.as_bytes()).unwrap_or("");
            let preview: String = text.chars().take(50).collect();
            eprintln!("{pad}{}: {:?}", node.kind(), preview);
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                print_tree(child, code, indent + 1);
            }
        }
        eprintln!("=== TREE STRUCTURE ===");
        print_tree(tree.root_node(), code, 0);
        eprintln!("=== END TREE ===");

        let summary = parser.parse_file_summary(code).expect("summary should parse");

        assert_eq!(summary.class_name, Some("MyPackage".to_string()));
        assert!(!summary.imports.is_empty());
        assert!(!summary.symbols.is_empty());
        assert!(summary.documentation.is_some());
    }
}
