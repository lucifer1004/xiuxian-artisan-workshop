use std::path::Path;

use crate::frontmatter::{extract_frontmatter, split_frontmatter};

use super::super::super::frontmatter::SkillFrontmatter;

pub(in crate::skills::scanner::scan) fn skill_name_from_path(skill_path: &Path) -> String {
    skill_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

pub(in crate::skills::scanner::scan) fn parse_skill_frontmatter(
    content: &str,
    skill_name: &str,
) -> Result<Option<SkillFrontmatter>, Box<dyn std::error::Error>> {
    let Some(frontmatter) = extract_frontmatter(content) else {
        log::warn!("No YAML frontmatter found in SKILL.md for: {skill_name}");
        return Ok(None);
    };

    let frontmatter_data: SkillFrontmatter = serde_yaml::from_str(&frontmatter)
        .map_err(|error| anyhow::anyhow!("Failed to parse SKILL.md frontmatter: {error}"))?;
    Ok(Some(frontmatter_data))
}

pub(in crate::skills::scanner::scan) fn parse_skill_frontmatter_strict(
    content: &str,
    skill_path: &Path,
) -> Result<SkillFrontmatter, Box<dyn std::error::Error>> {
    let Some(parts) = split_frontmatter(content) else {
        return Err(anyhow::anyhow!(
            "SKILL.md frontmatter is required but missing for {}",
            skill_path.display()
        ))
        .map_err(Into::into);
    };

    let yaml = parts.yaml;
    let value: serde_yaml::Value = serde_yaml::from_str(yaml).map_err(|error| {
        anyhow::anyhow!(
            "SKILL.md frontmatter parse failed for {}: {error}",
            skill_path.display()
        )
    })?;

    let mapping = value.as_mapping().ok_or_else(|| {
        anyhow::anyhow!(
            "SKILL.md frontmatter must contain nested `metadata` mapping at {}",
            skill_path.display()
        )
    })?;

    let name_key = serde_yaml::Value::String("name".to_string());
    let name_value = mapping.get(&name_key).ok_or_else(|| {
        anyhow::anyhow!(
            "SKILL.md frontmatter must contain top-level `name` at {}",
            skill_path.display()
        )
    })?;

    let is_name_valid = match name_value {
        serde_yaml::Value::String(value) => !value.trim().is_empty(),
        _ => false,
    };
    if !is_name_valid {
        return Err(anyhow::anyhow!(
            "SKILL.md frontmatter `name` must be a non-empty string at {}",
            skill_path.display()
        ))
        .map_err(Into::into);
    }

    let metadata_key = serde_yaml::Value::String("metadata".to_string());
    let metadata_value = mapping.get(&metadata_key).ok_or_else(|| {
        anyhow::anyhow!(
            "SKILL.md frontmatter must contain nested `metadata` mapping at {}",
            skill_path.display()
        )
    })?;
    if !matches!(metadata_value, serde_yaml::Value::Mapping(_)) {
        return Err(anyhow::anyhow!(
            "SKILL.md frontmatter `metadata` must be a mapping at {}",
            skill_path.display()
        ))
        .map_err(Into::into);
    }

    let frontmatter_data: SkillFrontmatter = serde_yaml::from_str(yaml).map_err(|error| {
        anyhow::anyhow!(
            "SKILL.md frontmatter parse failed for {}: {error}",
            skill_path.display()
        )
    })?;
    Ok(frontmatter_data)
}
