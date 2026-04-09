//! Modelica parsing with optional tree-sitter support.
//!
//! When the `tree-sitter` feature is enabled and the tree-sitter-modelica library
//! is available, this module uses the richer AST-based parser from xiuxian-ast.
//! Otherwise, it falls back to conservative line-based parsing.

use xiuxian_wendao_core::repo_intelligence::{ImportKind, RepoSymbolKind};

use super::types::{ParsedDeclaration, ParsedImport};

#[cfg(feature = "tree-sitter")]
use xiuxian_ast::TreeSitterModelicaParser;

/// Parse the package name from Modelica source.
pub(crate) fn parse_package_name(contents: &str) -> Option<String> {
    #[cfg(feature = "tree-sitter")]
    {
        if let Ok(mut parser) = TreeSitterModelicaParser::new()
            && let Ok(summary) = parser.parse_file_summary(contents)
            && let Some(name) = summary.class_name
        {
            return Some(name);
        }
    }
    contents
        .lines()
        .find_map(|line| parse_named_declaration(line, &["package"]))
}

/// Check if the source contains a Documentation annotation.
pub(crate) fn contains_documentation_annotation(contents: &str) -> bool {
    contents.contains("Documentation(")
}

/// Parse import statements from Modelica source.
pub(crate) fn parse_imports(contents: &str) -> Vec<ParsedImport> {
    #[cfg(feature = "tree-sitter")]
    {
        if let Ok(mut parser) = TreeSitterModelicaParser::new()
            && let Ok(summary) = parser.parse_file_summary(contents)
        {
            return summary
                .imports
                .into_iter()
                .enumerate()
                .map(|(index, import)| {
                    let kind = if import.alias.is_some() {
                        ImportKind::Module
                    } else {
                        ImportKind::Symbol
                    };
                    ParsedImport {
                        name: import.name,
                        alias: import.alias,
                        kind,
                        line_start: Some(index + 1),
                    }
                })
                .collect();
        }
    }
    parse_imports_fallback(contents)
}

fn parse_imports_fallback(contents: &str) -> Vec<ParsedImport> {
    contents
        .lines()
        .enumerate()
        .filter_map(|(line_num, line)| {
            let trimmed = line.trim();
            if !trimmed.starts_with("import ") {
                return None;
            }
            let import_content = trimmed.strip_prefix("import ")?;
            let import_content = import_content.trim_end_matches(';').trim();

            if let Some(eq_pos) = import_content.find('=') {
                let alias = import_content[..eq_pos].trim().to_string();
                let name = import_content[eq_pos + 1..].trim().to_string();
                return Some(ParsedImport {
                    name,
                    alias: Some(alias),
                    kind: ImportKind::Module,
                    line_start: Some(line_num + 1),
                });
            }

            let is_wildcard = import_content.ends_with(".*");
            let name = if is_wildcard {
                import_content.trim_end_matches(".*").trim().to_string()
            } else {
                import_content.to_string()
            };

            Some(ParsedImport {
                name,
                alias: None,
                kind: if is_wildcard {
                    ImportKind::Module
                } else {
                    ImportKind::Symbol
                },
                line_start: Some(line_num + 1),
            })
        })
        .collect()
}

/// Parse symbol declarations from Modelica source.
pub(crate) fn parse_symbol_declarations(contents: &str) -> Vec<ParsedDeclaration> {
    #[cfg(feature = "tree-sitter")]
    {
        if let Ok(mut parser) = TreeSitterModelicaParser::new()
            && let Ok(summary) = parser.parse_file_summary(contents)
        {
            return summary_to_declarations(summary);
        }
    }
    parse_symbol_declarations_fallback(contents)
}

#[cfg(feature = "tree-sitter")]
fn summary_to_declarations(summary: xiuxian_ast::ModelicaFileSummary) -> Vec<ParsedDeclaration> {
    summary
        .symbols
        .into_iter()
        .map(|symbol| ParsedDeclaration {
            name: symbol.name,
            kind: modelica_kind_to_repo_kind(symbol.kind),
            signature: symbol.signature.unwrap_or_default(),
            line_start: symbol.line_start,
            line_end: symbol.line_end,
            equations: symbol.equations,
        })
        .collect()
}

#[cfg(feature = "tree-sitter")]
fn modelica_kind_to_repo_kind(kind: xiuxian_ast::ModelicaSymbolKind) -> RepoSymbolKind {
    match kind {
        xiuxian_ast::ModelicaSymbolKind::Function => RepoSymbolKind::Function,
        xiuxian_ast::ModelicaSymbolKind::Class
        | xiuxian_ast::ModelicaSymbolKind::Package
        | xiuxian_ast::ModelicaSymbolKind::Connector
        | xiuxian_ast::ModelicaSymbolKind::Type => RepoSymbolKind::Type,
        xiuxian_ast::ModelicaSymbolKind::Constant => RepoSymbolKind::Constant,
    }
}

fn parse_symbol_declarations_fallback(contents: &str) -> Vec<ParsedDeclaration> {
    contents
        .lines()
        .enumerate()
        .filter_map(|(line_num, line)| {
            let trimmed = line.trim();
            if trimmed.starts_with("end ") || trimmed == "end;" {
                return None;
            }
            if let Some(name) = parse_named_declaration(trimmed, &["function"]) {
                return Some(ParsedDeclaration {
                    name,
                    kind: RepoSymbolKind::Function,
                    signature: trimmed.to_string(),
                    line_start: Some(line_num + 1),
                    line_end: Some(line_num + 1),
                    equations: Vec::new(),
                });
            }
            if let Some(name) =
                parse_named_declaration(trimmed, &["model", "record", "block", "connector", "type"])
            {
                return Some(ParsedDeclaration {
                    name,
                    kind: RepoSymbolKind::Type,
                    signature: trimmed.to_string(),
                    line_start: Some(line_num + 1),
                    line_end: Some(line_num + 1),
                    equations: Vec::new(),
                });
            }
            if let Some(name) = parse_named_declaration(trimmed, &["constant", "parameter"]) {
                return Some(ParsedDeclaration {
                    name,
                    kind: RepoSymbolKind::Constant,
                    signature: trimmed.to_string(),
                    line_start: Some(line_num + 1),
                    line_end: Some(line_num + 1),
                    equations: Vec::new(),
                });
            }
            None
        })
        .collect()
}

fn parse_named_declaration(line: &str, keywords: &[&str]) -> Option<String> {
    for keyword in keywords {
        let Some(suffix) = line.strip_prefix(keyword) else {
            continue;
        };
        let first = suffix.chars().next()?;
        if !first.is_whitespace() {
            continue;
        }
        let ident = suffix
            .trim_start()
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
            .next()?;
        if !ident.is_empty() {
            return Some(ident.to_string());
        }
    }
    None
}

#[cfg(test)]
#[path = "../../tests/unit/plugin/parsing.rs"]
mod tests;
