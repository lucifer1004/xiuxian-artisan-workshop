use std::path::Path;

use walkdir::WalkDir;

use crate::skills::metadata::{SkillMetadata, SkillStructure};

use super::super::SkillScanner;
use super::single::scan_single_skill_with_structure;

impl SkillScanner {
    /// Scan a directory for all valid skills.
    ///
    /// # Errors
    /// Returns an error if directory traversal or parsing fails.
    pub fn scan_all(
        &self,
        root: &Path,
        validate_struct: Option<&SkillStructure>,
    ) -> Result<Vec<SkillMetadata>, Box<dyn std::error::Error>> {
        if !root.is_dir() {
            return Err(anyhow::anyhow!("Root path is not a directory: {}", root.display()).into());
        }

        let mut skills = Vec::new();
        for entry in WalkDir::new(root)
            .min_depth(1)
            .max_depth(2)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_dir() {
                continue;
            }

            let skill_path = entry.path();
            if !skill_path.join("SKILL.md").exists() {
                continue;
            }

            if let Some(metadata) =
                scan_single_skill_with_structure(self, skill_path, validate_struct)?
            {
                skills.push(metadata);
            }
        }

        Ok(skills)
    }

    /// Scan a single skill directory and extract metadata.
    ///
    /// # Errors
    /// Returns an error if parsing or I/O fails.
    pub fn scan_skill(
        &self,
        skill_path: &Path,
        validate_struct: Option<&SkillStructure>,
    ) -> Result<Option<SkillMetadata>, Box<dyn std::error::Error>> {
        scan_single_skill_with_structure(self, skill_path, validate_struct)
    }
}
