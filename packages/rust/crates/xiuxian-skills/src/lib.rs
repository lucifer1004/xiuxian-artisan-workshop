//! Omni-Scanner - Unified scanning for skills and knowledge.
//!
//! This crate provides unified scanning capabilities:
//! - `skills/` - Scans SKILL.md and @`skill_command` scripts
//! - `knowledge/` - Scans knowledge documents with YAML frontmatter
//!
//! # Architecture
//!
//! ```text
//! xiuxian-skills/src/
//! ├── lib.rs              # Main module and exports
//! ├── frontmatter.rs      # Shared YAML frontmatter parsing
//! ├── skills/             # Skill scanning modules
//! │   ├── mod.rs
//! │   ├── metadata/        # Skill metadata types
//! │   ├── scanner/         # SKILL.md parser
//! │   ├── tools/           # @skill_command tool parser
//! │   ├── prompt/          # @prompt parser
//! │   ├── resource/        # @skill_resource parser
//! │   └── skill_command/   # @skill_command parsing utilities
//! └── knowledge/          # Knowledge document scanning
//!     ├── mod.rs
//!     ├── scanner/         # Knowledge document scanner
//!     └── types/           # Knowledge document models and enums
//! ```
//!
//! # YAML Frontmatter Support
//!
//! Both skills and knowledge use YAML frontmatter for metadata:
//!
//! ```yaml
//! ---
//! # For SKILL.md
//! name: git
//! description: Use when you need to work with git
//! metadata:
//!   routing_keywords: [commit, branch, log]
//!   intents: [version_control, repository_management]
//!
//! # For knowledge documents
//! title: Git Commit Best Practices
//! category: pattern
//! tags: [git, commit, best-practices]
//! ---

xiuxian_testing::crate_test_policy_source_harness!("../tests/unit/lib_policy.rs");

// ============================================================================
// Module Declarations
// ============================================================================

pub mod frontmatter;
pub mod knowledge;
pub mod skills;

// ============================================================================
// Re-exports from Skills Module
// ============================================================================

pub use skills::{
    CanonicalSkillPayload, CanonicalToolEntry, INTERNAL_SKILL_BINDING_PREFIX,
    InternalSkillBindingDescriptor, InternalSkillManifest, InternalSkillManifestScan,
    InternalSkillManifestSeed, InternalSkillMetadata, InternalSkillNativeAliasCompilation,
    InternalSkillNativeAliasCompileError, InternalSkillNativeAliasMountReport,
    InternalSkillNativeAliasSeed, InternalSkillNativeAliasSpec, InternalSkillWorkflowType,
    InternalToolAnnotationOverrides, compile_internal_skill_manifest_aliases,
    compile_internal_skill_native_alias, harden_internal_tool_annotations, internal_skill_bindings,
    metadata::{
        AssetRecord, DataRecord, DecoratorArgs, DocsAvailable, IndexToolEntry, ReferencePath,
        ReferenceRecord, ScanConfig, SkillIndexEntry, SkillMetadata, SkillStructure,
        SkillValidationPolicy, SkillValidationReport, SnifferRule, StructureItem, SyncReport,
        TemplateRecord, TestRecord, ToolAnnotations, ToolRecord, calculate_sync_ops,
    },
    parse_internal_skill_manifest_seed, resolve_internal_skill_binding_target,
    scanner::SkillScanner,
    tools::ToolsScanner,
    try_compile_internal_skill_native_alias,
};

// Re-export frontmatter helpers for external use
pub use frontmatter::{
    FrontmatterParts, extract_frontmatter, parse_and_validate_asset,
    parse_frontmatter_from_markdown, parse_typed_frontmatter_from_markdown, split_frontmatter,
};

// ============================================================================
// Re-exports from Knowledge Module
// ============================================================================

pub use knowledge::{
    scanner::KnowledgeScanner,
    types::{KnowledgeCategory, KnowledgeEntry, KnowledgeMetadata},
};

// ============================================================================
// JSON Schema Generation
// ============================================================================

/// Generate JSON Schema for `SkillIndexEntry`.
#[must_use]
pub fn skill_index_schema() -> String {
    let schema = schemars::schema_for!(SkillIndexEntry);
    serde_json::to_string_pretty(&schema).unwrap_or_default()
}

/// Generate JSON Schema for `KnowledgeEntry`.
#[must_use]
pub fn knowledge_entry_schema() -> String {
    let schema = schemars::schema_for!(KnowledgeEntry);
    serde_json::to_string_pretty(&schema).unwrap_or_default()
}

// ============================================================================
// Version
// ============================================================================

/// Crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
