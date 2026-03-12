use std::path::{Path, PathBuf};

use super::{SkillStructure, StructureItem, StructureItemKind};

const FORBIDDEN_SKILL_MD_LOGIC_TOKENS: &[&str] = &["{{", "{%", "{#"];

/// Validation report for one skill folder.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SkillValidationReport {
    /// Whether the inspected skill folder is valid.
    pub valid: bool,
    /// Human-readable issue list describing validation failures.
    pub issues: Vec<String>,
}

impl SkillValidationReport {
    #[must_use]
    fn from_issues(issues: Vec<String>) -> Self {
        Self {
            valid: issues.is_empty(),
            issues,
        }
    }
}

pub(super) fn validate_skill_path(
    structure: &SkillStructure,
    skill_path: &Path,
) -> SkillValidationReport {
    let mut issues = Vec::new();
    if !skill_path.exists() {
        issues.push(format!(
            "skill directory does not exist: {}",
            skill_path.display()
        ));
        return SkillValidationReport::from_issues(issues);
    }
    if !skill_path.is_dir() {
        issues.push(format!(
            "skill path is not a directory: {}",
            skill_path.display()
        ));
        return SkillValidationReport::from_issues(issues);
    }

    issues.extend(validate_items(skill_path, "required", &structure.required));
    if structure.validation.structure.strict_mode {
        issues.extend(validate_items(
            skill_path,
            "default(strict_mode)",
            &structure.default,
        ));
    }
    if structure.validation.structure.enforce_references_folder {
        let references_path = skill_path.join("references");
        if !(references_path.exists() && references_path.is_dir()) {
            issues.push(format!(
                "missing enforced references directory: {}",
                references_path.display()
            ));
        }
    }
    if structure.validation.frontmatter.prohibit_logic_in_skill_md {
        issues.extend(validate_skill_md_logic_policy(skill_path));
    }

    SkillValidationReport::from_issues(issues)
}

fn validate_items(skill_path: &Path, scope: &str, items: &[StructureItem]) -> Vec<String> {
    let mut issues = Vec::new();
    for item in items {
        let path = skill_path.join(item.path.as_str());
        if !path.exists() {
            issues.push(format!(
                "missing {} item `{}` at {}",
                scope,
                item.path,
                path.display()
            ));
            continue;
        }
        match item.item_kind() {
            StructureItemKind::File if !path.is_file() => {
                issues.push(format!(
                    "{} item `{}` must be file but found non-file at {}",
                    scope,
                    item.path,
                    path.display()
                ));
            }
            StructureItemKind::Dir if !path.is_dir() => {
                issues.push(format!(
                    "{} item `{}` must be directory but found non-directory at {}",
                    scope,
                    item.path,
                    path.display()
                ));
            }
            _ => {}
        }
    }
    issues
}

fn validate_skill_md_logic_policy(skill_path: &Path) -> Vec<String> {
    let mut issues = Vec::new();
    let skill_md_path = resolve_skill_md_path(skill_path);
    let Some(skill_md_path) = skill_md_path else {
        return issues;
    };
    let content = match std::fs::read_to_string(&skill_md_path) {
        Ok(content) => content,
        Err(error) => {
            issues.push(format!(
                "failed to read SKILL.md for logic validation at {}: {}",
                skill_md_path.display(),
                error
            ));
            return issues;
        }
    };
    for token in FORBIDDEN_SKILL_MD_LOGIC_TOKENS {
        if content.contains(token) {
            issues.push(format!(
                "SKILL.md contains forbidden logic token `{}` at {}",
                token,
                skill_md_path.display()
            ));
        }
    }
    issues
}

fn resolve_skill_md_path(skill_path: &Path) -> Option<PathBuf> {
    let uppercase = skill_path.join("SKILL.md");
    if uppercase.is_file() {
        return Some(uppercase);
    }
    let lowercase = skill_path.join("skill.md");
    if lowercase.is_file() {
        return Some(lowercase);
    }
    None
}
