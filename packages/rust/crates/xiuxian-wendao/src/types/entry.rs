use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Knowledge category enumeration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeCategory {
    #[serde(rename = "patterns")]
    /// Reusable pattern knowledge.
    Pattern,
    #[serde(rename = "solutions")]
    /// Problem-solution knowledge.
    Solution,
    #[serde(rename = "errors")]
    /// Error diagnosis and fixes.
    Error,
    #[serde(rename = "techniques")]
    /// Techniques and methods.
    Technique,
    #[serde(rename = "notes")]
    /// Free-form note content.
    #[default]
    Note,
    #[serde(rename = "references")]
    /// Reference material.
    Reference,
    #[serde(rename = "architecture")]
    /// Architecture design and decisions.
    Architecture,
    #[serde(rename = "workflows")]
    /// Process and workflow guidance.
    Workflow,
}

/// Knowledge entry struct representing a single knowledge piece.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeEntry {
    /// Unique identifier for the entry
    pub id: String,
    /// Human-readable title
    pub title: String,
    /// Main content/body of the knowledge entry
    pub content: String,
    /// Classification category
    pub category: KnowledgeCategory,
    /// Tags for filtering and search
    pub tags: Vec<String>,
    /// Original source file path or URL
    pub source: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp
    pub updated_at: DateTime<Utc>,
    /// Entry version for change tracking
    pub version: i32,
    /// Additional metadata for extensibility
    pub metadata: HashMap<String, serde_json::Value>,
}

impl KnowledgeEntry {
    /// Create a new `KnowledgeEntry` with required fields.
    #[must_use]
    pub fn new(id: String, title: String, content: String, category: KnowledgeCategory) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            content,
            category,
            tags: Vec::new(),
            source: None,
            created_at: now,
            updated_at: now,
            version: 1,
            metadata: HashMap::new(),
        }
    }

    /// Set tags for this entry.
    #[must_use]
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set source for this entry.
    #[must_use]
    pub fn with_source(mut self, source: Option<String>) -> Self {
        self.source = source;
        self
    }

    /// Add a tag to this entry.
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }
}
