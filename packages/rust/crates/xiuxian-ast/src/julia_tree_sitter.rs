//! Tree-sitter based Julia parser for conservative repository-entry extraction.

use std::collections::{BTreeMap, BTreeSet};

use regex::Regex;
use thiserror::Error;
use tree_sitter::{Language, Node, Parser};

/// Errors raised while parsing Julia source.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum JuliaParseError {
    /// The tree-sitter Julia language could not be initialized.
    #[error("failed to initialize tree-sitter Julia parser")]
    LanguageInit,
    /// Tree-sitter did not produce a syntax tree for the provided source.
    #[error("failed to parse Julia source")]
    ParseFailed,
    /// No root module declaration could be found.
    #[error("failed to find Julia root module declaration")]
    MissingModule,
}

/// Supported Julia symbol kinds for conservative AST extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JuliaSymbolKind {
    /// A function-like declaration.
    Function,
    /// A type-like declaration.
    Type,
}

/// One Julia symbol extracted from source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JuliaSymbol {
    /// Symbol display name.
    pub name: String,
    /// Normalized symbol kind.
    pub kind: JuliaSymbolKind,
    /// Optional signature snippet.
    pub signature: Option<String>,
}

/// One Julia import-like relation extracted from source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct JuliaImport {
    /// Imported module name.
    pub module: String,
    /// Whether the import was reexported.
    pub reexported: bool,
}

/// Target kinds supported by conservative Julia docstring extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum JuliaDocTargetKind {
    /// A module-level docstring.
    Module,
    /// A symbol-level docstring.
    Symbol,
}

/// One Julia docstring attachment extracted from source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct JuliaDocAttachment {
    /// Target display name.
    pub target_name: String,
    /// Normalized target kind.
    pub target_kind: JuliaDocTargetKind,
    /// Trimmed docstring contents.
    pub content: String,
}

/// Conservative Julia source summary for repository-entry analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JuliaFileSummary {
    /// Optional root module name declared in the file.
    pub module_name: Option<String>,
    /// Names exported from the file.
    pub exports: Vec<String>,
    /// Imported or reexported modules.
    pub imports: Vec<JuliaImport>,
    /// Directly declared functions and types.
    pub symbols: Vec<JuliaSymbol>,
    /// Conservative docstrings attached to directly declared symbols.
    pub docstrings: Vec<JuliaDocAttachment>,
    /// Literal `include("...")` paths referenced by the file.
    pub includes: Vec<String>,
}

/// Conservative Julia source summary for repository-entry analysis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JuliaSourceSummary {
    /// Root module name.
    pub module_name: String,
    /// Names exported from the root module.
    pub exports: Vec<String>,
    /// Imported or reexported modules.
    pub imports: Vec<JuliaImport>,
    /// Directly declared functions and types.
    pub symbols: Vec<JuliaSymbol>,
    /// Conservative docstrings attached to the module or directly declared symbols.
    pub docstrings: Vec<JuliaDocAttachment>,
    /// Literal `include("...")` paths referenced by the file.
    pub includes: Vec<String>,
}

/// Tree-sitter based Julia parser.
pub struct TreeSitterJuliaParser {
    parser: Parser,
}

impl TreeSitterJuliaParser {
    /// Create a new Julia parser.
    ///
    /// # Errors
    ///
    /// Returns [`JuliaParseError`] when the Julia tree-sitter language cannot
    /// be loaded into the parser.
    pub fn new() -> Result<Self, JuliaParseError> {
        let language: Language = tree_sitter_julia::LANGUAGE.into();
        let mut parser = Parser::new();
        parser
            .set_language(&language)
            .map_err(|_| JuliaParseError::LanguageInit)?;
        Ok(Self { parser })
    }

    /// Parse one Julia source file into a conservative summary.
    ///
    /// # Errors
    ///
    /// Returns [`JuliaParseError`] when parsing fails or when no root module
    /// declaration can be found.
    pub fn parse_summary(&mut self, code: &str) -> Result<JuliaSourceSummary, JuliaParseError> {
        let summary = self.parse_file_summary(code)?;
        let module_name = summary.module_name.ok_or(JuliaParseError::MissingModule)?;

        Ok(JuliaSourceSummary {
            module_name,
            exports: summary.exports,
            imports: summary.imports,
            symbols: summary.symbols,
            docstrings: summary.docstrings,
            includes: summary.includes,
        })
    }

    /// Parse one Julia source file into a conservative summary without
    /// requiring a root `module` declaration.
    ///
    /// # Errors
    ///
    /// Returns [`JuliaParseError`] when parsing fails.
    pub fn parse_file_summary(&mut self, code: &str) -> Result<JuliaFileSummary, JuliaParseError> {
        let tree = self
            .parser
            .parse(code, None)
            .ok_or(JuliaParseError::ParseFailed)?;

        let mut module_name = None;
        let mut exports = BTreeSet::new();
        let mut imports = BTreeMap::new();
        let mut symbols = BTreeMap::new();

        collect_summary(
            tree.root_node(),
            code,
            &mut module_name,
            &mut exports,
            &mut imports,
            &mut symbols,
        );

        for raw_line in code.lines() {
            if let Some(import) = parse_reexport_using_line(raw_line) {
                imports.insert(import.module.clone(), import);
            }
        }

        let resolved_module_name = module_name.or_else(|| parse_module_name(code));

        Ok(JuliaFileSummary {
            module_name: resolved_module_name.clone(),
            exports: exports.into_iter().collect(),
            imports: imports.into_values().collect(),
            symbols: symbols.into_values().collect(),
            docstrings: parse_docstrings(code, resolved_module_name.as_deref()),
            includes: parse_include_literals(code),
        })
    }
}

fn collect_summary(
    node: Node<'_>,
    code: &str,
    module_name: &mut Option<String>,
    exports: &mut BTreeSet<String>,
    imports: &mut BTreeMap<String, JuliaImport>,
    symbols: &mut BTreeMap<String, JuliaSymbol>,
) {
    let node_text = node.utf8_text(code.as_bytes()).unwrap_or("");

    match node.kind() {
        "module_definition" => {
            if module_name.is_none() {
                *module_name = parse_module_name(node_text);
            }
        }
        "export_statement" => {
            exports.extend(parse_export_names(node_text));
        }
        "using_statement" | "import_statement" | "selected_import" => {
            if let Some(import) = parse_plain_import(node_text) {
                upsert_import(imports, import);
            }
        }
        "function_definition" => {
            if let Some(symbol) = parse_long_function(node_text) {
                upsert_symbol(symbols, symbol);
            }
        }
        "assignment" => {
            if let Some(symbol) = parse_short_function(node_text) {
                upsert_symbol(symbols, symbol);
            }
        }
        "struct_definition" | "abstract_definition" | "primitive_definition" => {
            if let Some(symbol) = parse_type_definition(node_text) {
                upsert_symbol(symbols, symbol);
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_summary(child, code, module_name, exports, imports, symbols);
    }
}

fn upsert_import(imports: &mut BTreeMap<String, JuliaImport>, import: JuliaImport) {
    match imports.get(&import.module) {
        Some(existing) if !existing.reexported && import.reexported => {
            imports.insert(import.module.clone(), import);
        }
        None => {
            imports.insert(import.module.clone(), import);
        }
        Some(_) => {}
    }
}

fn upsert_symbol(symbols: &mut BTreeMap<String, JuliaSymbol>, symbol: JuliaSymbol) {
    let key = symbol.name.clone();
    match symbols.get(&key) {
        Some(existing)
            if existing.kind == JuliaSymbolKind::Type
                || symbol.kind == JuliaSymbolKind::Function =>
        {
            symbols.insert(key, symbol);
        }
        None => {
            symbols.insert(key, symbol);
        }
        Some(_) => {}
    }
}

fn parse_module_name(text: &str) -> Option<String> {
    let regex = Regex::new(r"(?m)^\s*module\s+([A-Za-z_][A-Za-z0-9_]*)\b").ok()?;
    regex
        .captures(text)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str().to_string())
}

fn parse_export_names(text: &str) -> Vec<String> {
    let regex = Regex::new(r"(?m)^\s*export\s+(.+)$").ok();
    let Some(regex) = regex else {
        return Vec::new();
    };
    regex
        .captures(text)
        .and_then(|captures| captures.get(1))
        .map(|segment| split_comma_names(segment.as_str()))
        .unwrap_or_default()
}

fn parse_plain_import(text: &str) -> Option<JuliaImport> {
    let regex = Regex::new(r"(?m)^\s*(?:using|import)\s+(.+)$").ok()?;
    let captures = regex.captures(text)?;
    let raw = captures.get(1)?.as_str();
    let module = split_comma_names(raw).into_iter().next()?;
    Some(JuliaImport {
        module: module
            .split(':')
            .next()
            .unwrap_or(&module)
            .trim()
            .to_string(),
        reexported: false,
    })
}

fn parse_reexport_using_line(line: &str) -> Option<JuliaImport> {
    let regex = Regex::new(r"^\s*@reexport\s+using\s+(.+)$").ok()?;
    let captures = regex.captures(strip_inline_comment(line))?;
    let raw = captures.get(1)?.as_str();
    let module = split_comma_names(raw).into_iter().next()?;
    Some(JuliaImport {
        module: module
            .split(':')
            .next()
            .unwrap_or(&module)
            .trim()
            .to_string(),
        reexported: true,
    })
}

fn parse_long_function(text: &str) -> Option<JuliaSymbol> {
    let regex = Regex::new(r"(?m)^\s*function\s+([A-Za-z_][A-Za-z0-9_!]*)(?:\{|[(!])").ok()?;
    let captures = regex.captures(text)?;
    let name = captures.get(1)?.as_str().to_string();
    let signature = text
        .lines()
        .next()
        .map(str::trim)
        .map(|line| line.trim_start_matches("function").trim().to_string());
    Some(JuliaSymbol {
        name,
        kind: JuliaSymbolKind::Function,
        signature,
    })
}

fn parse_short_function(text: &str) -> Option<JuliaSymbol> {
    let regex = Regex::new(r"(?m)^\s*([A-Za-z_][A-Za-z0-9_!]*)\s*\([^=\n]*\)\s*=").ok()?;
    let captures = regex.captures(text)?;
    let name = captures.get(1)?.as_str().trim().to_string();
    if is_reserved_identifier(name.as_str()) {
        return None;
    }
    let signature = text.lines().next().map(str::trim).map(str::to_string);
    Some(JuliaSymbol {
        name,
        kind: JuliaSymbolKind::Function,
        signature,
    })
}

fn parse_type_definition(text: &str) -> Option<JuliaSymbol> {
    let regex = Regex::new(
        r"(?m)^\s*(?:abstract\s+type|primitive\s+type|mutable\s+struct|struct)\s+([A-Za-z_][A-Za-z0-9_]*)",
    )
    .ok()?;
    let captures = regex.captures(text)?;
    Some(JuliaSymbol {
        name: captures.get(1)?.as_str().to_string(),
        kind: JuliaSymbolKind::Type,
        signature: None,
    })
}

fn strip_inline_comment(line: &str) -> &str {
    line.split_once('#').map_or(line, |(prefix, _)| prefix)
}

fn split_comma_names(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect()
}

fn is_reserved_identifier(candidate: &str) -> bool {
    matches!(
        candidate,
        "if" | "for"
            | "while"
            | "let"
            | "begin"
            | "quote"
            | "macro"
            | "module"
            | "baremodule"
            | "struct"
            | "mutable"
            | "abstract"
            | "primitive"
            | "return"
    )
}

fn parse_docstrings(code: &str, module_name: Option<&str>) -> Vec<JuliaDocAttachment> {
    let mut docstrings = BTreeMap::new();
    let lines = code.lines().collect::<Vec<_>>();
    let mut line_index = 0;
    while line_index < lines.len() {
        let Some((content, end_index)) = parse_triple_quoted_docstring(&lines, line_index) else {
            line_index += 1;
            continue;
        };

        let mut next_index = end_index + 1;
        while next_index < lines.len() && lines[next_index].trim().is_empty() {
            next_index += 1;
        }

        let Some((target_kind, target_name)) =
            parse_docstring_target(lines.get(next_index).copied(), module_name)
        else {
            line_index = end_index + 1;
            continue;
        };

        docstrings.insert(
            (target_kind, target_name.clone()),
            JuliaDocAttachment {
                target_name,
                target_kind,
                content,
            },
        );
        line_index = end_index + 1;
    }

    docstrings.into_values().collect()
}

fn parse_include_literals(code: &str) -> Vec<String> {
    let Ok(regex) = Regex::new(r#"(?m)^\s*include\(\s*"([^"]+)"\s*\)"#) else {
        return Vec::new();
    };

    regex
        .captures_iter(code)
        .filter_map(|captures| captures.get(1))
        .map(|value| value.as_str().trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

fn parse_triple_quoted_docstring(lines: &[&str], start_index: usize) -> Option<(String, usize)> {
    let start_line = lines.get(start_index)?.trim_start();
    if !start_line.starts_with("\"\"\"") {
        return None;
    }

    let opening_remainder = start_line.trim_start_matches("\"\"\"");
    if let Some((content, _)) = opening_remainder.split_once("\"\"\"") {
        return Some((content.trim().to_string(), start_index));
    }

    let mut content_lines = Vec::new();
    if !opening_remainder.trim().is_empty() {
        content_lines.push(opening_remainder.trim_end().to_string());
    }

    for (offset, line) in lines.iter().enumerate().skip(start_index + 1) {
        let trimmed = line.trim_end();
        if let Some((content, _)) = trimmed.split_once("\"\"\"") {
            if !content.trim().is_empty() {
                content_lines.push(content.trim_end().to_string());
            }
            let content = content_lines.join("\n").trim().to_string();
            return (!content.is_empty()).then_some((content, offset));
        }
        content_lines.push(trimmed.to_string());
    }

    None
}

fn parse_docstring_target(
    line: Option<&str>,
    module_name: Option<&str>,
) -> Option<(JuliaDocTargetKind, String)> {
    let line = line?.trim();
    if line.is_empty() {
        return None;
    }
    if let Some(name) = parse_module_name(line) {
        return module_name
            .filter(|module_name| name == *module_name)
            .map(|_| (JuliaDocTargetKind::Module, name));
    }
    if let Some(symbol) = parse_long_function(line) {
        return Some((JuliaDocTargetKind::Symbol, symbol.name));
    }
    if let Some(symbol) = parse_short_function(line) {
        return Some((JuliaDocTargetKind::Symbol, symbol.name));
    }
    if let Some(symbol) = parse_type_definition(line) {
        return Some((JuliaDocTargetKind::Symbol, symbol.name));
    }
    None
}

#[cfg(test)]
mod tests {
    use insta::assert_debug_snapshot;

    use super::TreeSitterJuliaParser;

    #[test]
    fn parse_summary_extracts_root_julia_concepts() {
        let mut parser = TreeSitterJuliaParser::new().expect("parser should initialize");
        let summary = parser
            .parse_summary(
                r#"module SamplePkg

export solve, Problem
using LinearAlgebra
@reexport using SciMLBase

"""
Problem docs.
"""
struct Problem
    x::Int
end

"""
Solve docs.
"""
function solve(problem::Problem)
    problem.x
end

"""
fastsolve docs.
"""
fastsolve(problem::Problem) = problem.x

end
"#,
            )
            .expect("summary should parse");

        assert_debug_snapshot!("julia_root_summary", summary);
    }

    #[test]
    fn parse_file_summary_extracts_includes_without_module() {
        let mut parser = TreeSitterJuliaParser::new().expect("parser should initialize");
        let summary = parser
            .parse_file_summary(
                r#"
"""
Fast solve docs.
"""
fastsolve(problem::Problem) = problem.x

include("nested/extra.jl")
"#,
            )
            .expect("file summary should parse");

        assert_debug_snapshot!("julia_file_summary", summary);
    }
}
