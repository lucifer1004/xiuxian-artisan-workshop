use serde::{Deserialize, Serialize};

/// Parsed YAML frontmatter from a markdown note.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteFrontmatter {
    /// Document title from frontmatter.
    pub title: Option<String>,
    /// Human-readable description.
    pub description: Option<String>,
    /// Skill name (for SKILL.md files).
    pub name: Option<String>,
    /// Document category (e.g. "pattern", "architecture").
    pub category: Option<String>,
    /// Tags for discovery and filtering.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Routing keywords from `metadata.routing_keywords`.
    #[serde(default)]
    pub routing_keywords: Vec<String>,
    /// Intent descriptions from `metadata.intents`.
    #[serde(default)]
    pub intents: Vec<String>,
}
