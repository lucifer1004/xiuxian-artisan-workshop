use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use super::ResourceScanner;
use super::build::build_resource_records;
use crate::skills::metadata::ResourceRecord;

impl ResourceScanner {
    /// Scan a scripts directory for @`skill_resource` decorated functions.
    ///
    /// # Arguments
    ///
    /// * `scripts_dir` - Path to the scripts directory
    /// * `skill_name` - Name of the parent skill
    ///
    /// # Returns
    ///
    /// A vector of `ResourceRecord` objects.
    ///
    /// # Errors
    ///
    /// Returns an error when `skill_name` is empty.
    pub fn scan(
        &self,
        scripts_dir: &Path,
        skill_name: &str,
    ) -> Result<Vec<ResourceRecord>, Box<dyn std::error::Error>> {
        let _ = self;
        if skill_name.trim().is_empty() {
            return Err("skill_name cannot be empty".into());
        }
        let mut resources = Vec::new();

        if !scripts_dir.exists() {
            log::debug!("Scripts directory not found: {}", scripts_dir.display());
            return Ok(resources);
        }

        for entry in WalkDir::new(scripts_dir)
            .follow_links(true)
            .sort_by_file_name()
        {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    log::warn!("Error walking directory {}: {e}", scripts_dir.display());
                    continue;
                }
            };

            let path = entry.path();
            if !entry.file_type().is_file() {
                continue;
            }

            // Only scan Python files, skip __init__.py
            if path.extension().map(|e| e.to_string_lossy()) != Some("py".into()) {
                continue;
            }
            if path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("__"))
            {
                continue;
            }

            match Self::scan_file(path, skill_name) {
                Ok(file_resources) => resources.extend(file_resources),
                Err(e) => log::warn!("Error scanning {}: {e}", path.display()),
            }
        }

        log::debug!(
            "ResourceScanner: Found {} @skill_resource functions in {}",
            resources.len(),
            scripts_dir.display()
        );

        Ok(resources)
    }

    /// Scan multiple files for @`skill_resource` decorated functions.
    ///
    /// Used for testing.
    ///
    /// # Errors
    ///
    /// Returns an error when `skill_name` is empty.
    pub fn scan_paths(
        &self,
        files: &[(String, String)],
        skill_name: &str,
    ) -> Result<Vec<ResourceRecord>, Box<dyn std::error::Error>> {
        let _ = self;
        if skill_name.trim().is_empty() {
            return Err("skill_name cannot be empty".into());
        }
        let mut all_resources = Vec::new();

        for (file_path, content) in files {
            let file_hash = hex::encode(Sha256::digest(content.as_bytes()));
            let resources = build_resource_records(content, file_path, skill_name, &file_hash);
            all_resources.extend(resources);
        }

        Ok(all_resources)
    }

    /// Scan a single file for @`skill_resource` decorated functions.
    fn scan_file(
        path: &Path,
        skill_name: &str,
    ) -> Result<Vec<ResourceRecord>, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let file_hash = hex::encode(Sha256::digest(content.as_bytes()));
        let file_path = path.to_string_lossy().to_string();
        Ok(build_resource_records(
            &content, &file_path, skill_name, &file_hash,
        ))
    }
}
