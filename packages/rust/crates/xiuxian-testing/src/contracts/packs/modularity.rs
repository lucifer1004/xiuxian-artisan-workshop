//! Deterministic modularity checks over Rust source artifacts.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{Value, json};

use super::super::model::{
    ArtifactKind, CollectedArtifact, CollectedArtifacts, CollectionContext, ContractFinding,
    EvidenceKind, FindingMode, FindingSeverity,
};
use super::super::rule_pack::{RulePack, RulePackDescriptor};

const PACK_ID: &str = "modularity";
const MOD_R001: &str = "MOD-R001";
const MOD_R002: &str = "MOD-R002";
const MOD_R003: &str = "MOD-R003";

/// Deterministic V1 modularity checks over Rust source files.
#[derive(Debug, Default, Clone, Copy)]
pub struct ModularityRulePack;

impl RulePack for ModularityRulePack {
    fn descriptor(&self) -> RulePackDescriptor {
        RulePackDescriptor {
            id: PACK_ID,
            version: "v1",
            domains: &["modularity", "architecture", "rust"],
            default_mode: FindingMode::Deterministic,
        }
    }

    fn collect(&self, ctx: &CollectionContext) -> Result<CollectedArtifacts> {
        let Some(source_root) = resolve_source_root(ctx) else {
            return Ok(CollectedArtifacts::default());
        };

        let mut files = Vec::new();
        collect_rust_files(&source_root, &mut files)?;
        files.sort();

        let mut artifacts = CollectedArtifacts::default();
        artifacts
            .metadata
            .insert("source_root".to_string(), source_root.display().to_string());
        for file in files {
            let text = fs::read_to_string(&file)
                .with_context(|| format!("failed to read Rust source file {}", file.display()))?;
            artifacts.push(CollectedArtifact {
                id: artifact_id(ctx.workspace_root.as_deref(), &file),
                kind: ArtifactKind::SourceFile,
                path: Some(file),
                content: json!({ "text": text }),
                labels: BTreeMap::from([("pack".to_string(), PACK_ID.to_string())]),
            });
        }

        Ok(artifacts)
    }

    fn evaluate(&self, artifacts: &CollectedArtifacts) -> Result<Vec<ContractFinding>> {
        let mut findings = Vec::new();
        for artifact in &artifacts.artifacts {
            if artifact.kind != ArtifactKind::SourceFile {
                continue;
            }
            let Some(path) = artifact.path.as_deref() else {
                continue;
            };
            let Some(text) = artifact.content.get("text").and_then(Value::as_str) else {
                continue;
            };

            findings.extend(check_mod_interface_only(path, text));
            findings.extend(check_visibility_boundary(path, text));
            findings.extend(check_public_result_error_docs(path, text));
        }
        Ok(findings)
    }
}

fn resolve_source_root(ctx: &CollectionContext) -> Option<PathBuf> {
    let workspace_root = ctx.workspace_root.as_ref()?;
    if let Some(crate_name) = ctx
        .crate_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        let canonical = workspace_root
            .join("packages")
            .join("rust")
            .join("crates")
            .join(crate_name)
            .join("src");
        if canonical.is_dir() {
            return Some(canonical);
        }

        let fallback = workspace_root.join(crate_name).join("src");
        if fallback.is_dir() {
            return Some(fallback);
        }
    }

    let workspace_src = workspace_root.join("src");
    if workspace_src.is_dir() {
        return Some(workspace_src);
    }
    None
}

fn collect_rust_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(root)
        .with_context(|| format!("failed to read source directory {}", root.display()))?;
    for entry in entries {
        let entry = entry
            .with_context(|| format!("failed to traverse source directory {}", root.display()))?;
        let path = entry.path();
        if path.is_dir() {
            collect_rust_files(&path, files)?;
            continue;
        }
        if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn artifact_id(workspace_root: Option<&Path>, path: &Path) -> String {
    if let Some(root) = workspace_root {
        return path.strip_prefix(root).ok().map_or_else(
            || path.to_string_lossy().into_owned(),
            |relative| relative.to_string_lossy().into_owned(),
        );
    }
    path.to_string_lossy().into_owned()
}

fn check_mod_interface_only(path: &Path, text: &str) -> Vec<ContractFinding> {
    if path.file_name().and_then(|name| name.to_str()) != Some("mod.rs") {
        return Vec::new();
    }

    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if is_ignorable_line(trimmed) || is_interface_only_line(trimmed) {
            continue;
        }

        let mut finding = base_finding(
            MOD_R001,
            FindingSeverity::Error,
            path,
            "mod.rs should remain interface-only",
            format!(
                "`{}` contains implementation logic at line {}.",
                display_path(path),
                index + 1
            ),
        );
        finding.why_it_matters =
            "Interface modules should expose structure only; implementation in `mod.rs` blurs module boundaries.".to_string();
        finding.remediation = "Move implementation into dedicated submodules and keep `mod.rs` to re-exports and module declarations.".to_string();
        finding
            .examples
            .good
            .push("`mod.rs` with `mod foo;` + `pub use foo::Type;` only.".to_string());
        finding
            .examples
            .bad
            .push("`mod.rs` defines business functions or concrete state.".to_string());
        finding.evidence.push(FindingEvidenceEntry::source_span(
            path,
            index + 1,
            "Non-interface statement found in `mod.rs`.".to_string(),
        ));
        return vec![finding];
    }

    Vec::new()
}

fn check_visibility_boundary(path: &Path, text: &str) -> Vec<ContractFinding> {
    let file_name = path.file_name().and_then(|name| name.to_str());
    if matches!(file_name, Some("lib.rs" | "main.rs" | "mod.rs")) {
        return Vec::new();
    }
    if path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        == Some("src")
    {
        return Vec::new();
    }

    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if !is_broad_public_item(trimmed) {
            continue;
        }

        let mut finding = base_finding(
            MOD_R002,
            FindingSeverity::Warning,
            path,
            "Public visibility may be broader than required",
            format!(
                "Public item at line {} in `{}` may need narrower visibility.",
                index + 1,
                display_path(path)
            ),
        );
        finding.why_it_matters = "Overly broad visibility weakens module boundaries and increases accidental API surface.".to_string();
        finding.remediation = "Use `pub(crate)` (or narrower) for internal items unless the symbol is intentionally part of the external crate API.".to_string();
        finding
            .examples
            .good
            .push("`pub(crate) struct InternalState` for crate-internal wiring.".to_string());
        finding
            .examples
            .bad
            .push("`pub struct` used only by sibling modules inside the same crate.".to_string());
        finding.evidence.push(FindingEvidenceEntry::source_span(
            path,
            index + 1,
            line.trim().to_string(),
        ));
        return vec![finding];
    }

    Vec::new()
}

fn check_public_result_error_docs(path: &Path, text: &str) -> Vec<ContractFinding> {
    let lines = text.lines().collect::<Vec<_>>();
    let mut findings = Vec::new();

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if !is_public_result_signature(trimmed) {
            continue;
        }
        if has_errors_doc(&lines, index) {
            continue;
        }

        let mut finding = base_finding(
            MOD_R003,
            FindingSeverity::Error,
            path,
            "Public Result API is missing `# Errors` docs",
            format!(
                "Public function at line {} in `{}` returns `Result` without `# Errors` documentation.",
                index + 1,
                display_path(path)
            ),
        );
        finding.why_it_matters =
            "Error contracts are part of public API semantics and should be documented explicitly."
                .to_string();
        finding.remediation = "Add a doc-comment section with `# Errors` that describes failure conditions for this API.".to_string();
        finding
            .examples
            .good
            .push("`/// # Errors` followed by concrete failure scenarios.".to_string());
        finding
            .examples
            .bad
            .push("Public `Result` function with no error contract docs.".to_string());
        finding.evidence.push(FindingEvidenceEntry::source_span(
            path,
            index + 1,
            line.trim().to_string(),
        ));
        findings.push(finding);
    }

    findings
}

fn has_errors_doc(lines: &[&str], signature_index: usize) -> bool {
    let mut saw_doc = false;
    for cursor in (0..signature_index).rev() {
        let trimmed = lines[cursor].trim();
        if trimmed.is_empty() {
            if saw_doc {
                break;
            }
            continue;
        }
        if trimmed.starts_with("#[") {
            continue;
        }
        if trimmed.starts_with("///") || trimmed.starts_with("//!") {
            saw_doc = true;
            if trimmed.contains("# Errors") {
                return true;
            }
            continue;
        }
        break;
    }
    false
}

fn is_ignorable_line(line: &str) -> bool {
    line.is_empty()
        || line.starts_with("//")
        || line.starts_with("/*")
        || line.starts_with('*')
        || line.starts_with("*/")
        || line.starts_with("#[")
}

fn is_interface_only_line(line: &str) -> bool {
    if !line.ends_with(';') {
        return false;
    }
    line.starts_with("mod ")
        || line.starts_with("pub mod ")
        || line.starts_with("pub(crate) mod ")
        || line.starts_with("pub use ")
        || line.starts_with("pub(crate) use ")
}

fn is_broad_public_item(line: &str) -> bool {
    line.starts_with("pub ")
        && !line.starts_with("pub(crate)")
        && !line.starts_with("pub(super)")
        && !line.starts_with("pub(in ")
        && !line.starts_with("pub use ")
        && (line.starts_with("pub fn ")
            || line.starts_with("pub async fn ")
            || line.starts_with("pub struct ")
            || line.starts_with("pub enum ")
            || line.starts_with("pub trait ")
            || line.starts_with("pub type ")
            || line.starts_with("pub const ")
            || line.starts_with("pub static "))
}

fn is_public_result_signature(line: &str) -> bool {
    let is_public_function = line.starts_with("pub fn ") || line.starts_with("pub async fn ");
    let returns_result = line.contains("->") && line.contains("Result<");
    is_public_function && returns_result
}

fn base_finding(
    rule_id: &str,
    severity: FindingSeverity,
    path: &Path,
    title: impl Into<String>,
    summary: impl Into<String>,
) -> ContractFinding {
    let mut finding = ContractFinding::new(
        rule_id,
        PACK_ID,
        severity,
        FindingMode::Deterministic,
        title,
        summary,
    );
    finding
        .labels
        .insert("domain".to_string(), "modularity".to_string());
    finding
        .labels
        .insert("path".to_string(), display_path(path));
    finding
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

struct FindingEvidenceEntry;

impl FindingEvidenceEntry {
    fn source_span(
        path: &Path,
        line_number: usize,
        message: String,
    ) -> super::super::model::FindingEvidence {
        super::super::model::FindingEvidence {
            kind: EvidenceKind::SourceSpan,
            path: Some(path.to_path_buf()),
            locator: Some(format!("line:{line_number}")),
            message,
        }
    }
}
