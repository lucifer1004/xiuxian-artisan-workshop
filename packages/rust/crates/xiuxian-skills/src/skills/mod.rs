//! Skills Scanner Module
//!
//! Scans skill directories for SKILL.md and @`skill_command` scripts.

pub mod canonical;
/// Pure internal-skill alias compilation helpers shared by runtime consumers.
pub mod internal_native;
pub mod metadata;
pub mod prompt;
pub mod resource;
pub mod scanner;
pub mod skill_command;
pub mod tools;

// Re-export common types from submodules
pub use canonical::{CanonicalSkillPayload, CanonicalToolEntry};
pub use internal_native::{
    INTERNAL_SKILL_BINDING_PREFIX, InternalSkillBindingDescriptor, InternalSkillManifest,
    InternalSkillManifestScan, InternalSkillManifestSeed, InternalSkillMetadata,
    InternalSkillNativeAliasCompilation, InternalSkillNativeAliasCompileError,
    InternalSkillNativeAliasMountReport, InternalSkillNativeAliasSeed,
    InternalSkillNativeAliasSpec, InternalSkillWorkflowType, InternalToolAnnotationOverrides,
    compile_internal_skill_manifest_aliases, compile_internal_skill_native_alias,
    harden_internal_tool_annotations, internal_skill_bindings, parse_internal_skill_manifest_seed,
    resolve_internal_skill_binding_target, try_compile_internal_skill_native_alias,
};
pub use metadata::{
    AssetRecord, DataRecord, DecoratorArgs, DocsAvailable, IndexToolEntry, PromptRecord,
    ReferencePath, ReferenceRecord, ResourceRecord, ScanConfig, SkillIndexEntry, SkillMetadata,
    SkillStructure, SkillValidationPolicy, SkillValidationReport, SnifferRule, StructureItem,
    SyncReport, TemplateRecord, TestRecord, ToolAnnotations, ToolRecord, calculate_sync_ops,
};
pub use prompt::PromptScanner;
pub use resource::ResourceScanner;
pub use scanner::SkillScanner;
pub use tools::ToolsScanner;
