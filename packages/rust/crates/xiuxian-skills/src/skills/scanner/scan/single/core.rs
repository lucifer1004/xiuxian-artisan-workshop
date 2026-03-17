use std::fs;
use std::path::Path;

use crate::skills::metadata::{SkillMetadata, SkillStructure};
use crate::skills::scanner::SkillScanner;
use crate::skills::scanner::references::validate_references_strict;
use crate::skills::scanner::scan::parse::{
    build_skill_metadata, parse_skill_frontmatter, parse_skill_frontmatter_strict,
    skill_name_from_path,
};

pub fn scan_single_skill_with_structure(
    scanner: &SkillScanner,
    skill_path: &Path,
    structure: Option<&SkillStructure>,
) -> Result<Option<SkillMetadata>, Box<dyn std::error::Error>> {
    let skill_md_path = skill_path.join("SKILL.md");
    if !skill_md_path.exists() {
        log::debug!("SKILL.md not found for skill: {}", skill_path.display());
        return Ok(None);
    }

    warn_if_structure_mismatch(skill_path, structure);

    let content = fs::read_to_string(&skill_md_path)?;
    let skill_name = skill_name_from_path(skill_path);

    let metadata = if let Some(structure) = structure {
        let policy = structure.validation_policy();
        if policy.frontmatter.require_yaml_frontmatter {
            let frontmatter = parse_skill_frontmatter_strict(&content, skill_path)?;
            build_skill_metadata(skill_name, Some(frontmatter))
        } else {
            let frontmatter = parse_skill_frontmatter(&content, &skill_name)?;
            build_skill_metadata(skill_name, frontmatter)
        }
    } else {
        let frontmatter = parse_skill_frontmatter(&content, &skill_name)?;
        build_skill_metadata(skill_name, frontmatter)
    };

    if let Some(structure) = structure {
        let policy = structure.validation_policy();
        if policy.structure.enforce_references_folder {
            validate_references_strict(skill_path).map_err(|error| anyhow::anyhow!(error))?;
        }
    }

    let final_metadata = metadata;
    // We don't have references field on SkillMetadata directly in the struct definition seen before
    // If it was there, it would be used. Assuming scanner.scan_references might not be there.
    let _ = scanner;

    Ok(Some(final_metadata))
}

fn warn_if_structure_mismatch(skill_path: &Path, structure: Option<&SkillStructure>) {
    if let Some(structure) = structure
        && !structure.validate_skill_path(skill_path).valid
    {
        log::warn!(
            "Skill structure mismatch at {}: structure validation failed",
            skill_path.display()
        );
    }
}
