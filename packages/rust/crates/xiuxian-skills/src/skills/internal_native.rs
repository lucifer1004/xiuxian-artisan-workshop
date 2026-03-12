//! Internal skill manifest and alias compilation helpers.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

use super::metadata::ToolAnnotations;

/// Prefix used for internal skill bindings.
pub const INTERNAL_SKILL_BINDING_PREFIX: &str = "internal://";

/// Workflow type for internal skill execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InternalSkillWorkflowType {
    /// Execution driven by the `Qianji` orchestration engine.
    QianjiFlow,
    /// Direct dispatch to a native tool provider.
    NativeDispatch,
    /// Generic native tool execution.
    Native,
    /// Execution managed by an autonomous agent.
    Agentic,
}

impl Default for InternalSkillWorkflowType {
    fn default() -> Self {
        Self::QianjiFlow
    }
}

impl InternalSkillWorkflowType {
    /// Parse a workflow type from raw manifest values.
    #[must_use]
    pub fn from_raw(raw: Option<&str>) -> Self {
        let normalized = raw.unwrap_or("qianji_flow").trim().to_ascii_lowercase();
        match normalized.as_str() {
            "qianji_flow" | "qianji-flow" | "qianji" | "flow" | "workflow" => Self::QianjiFlow,
            "native_dispatch" | "native-dispatch" => Self::NativeDispatch,
            "native" => Self::Native,
            "agentic" => Self::Agentic,
            _ => Self::QianjiFlow,
        }
    }

    /// Return a stable string form for serialization.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::QianjiFlow => "qianji_flow",
            Self::NativeDispatch => "native_dispatch",
            Self::Native => "native",
            Self::Agentic => "agentic",
        }
    }
}

/// Free-form metadata attached to internal skills.
pub type InternalSkillMetadata = Value;

/// Annotation overrides applied to internal tool bindings.
pub type InternalToolAnnotationOverrides = ToolAnnotations;

/// Descriptor mapping an internal binding id to a runtime tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalSkillBindingDescriptor {
    /// Internal unique binding identifier.
    pub internal_id: String,
    /// Target native tool name.
    pub target_tool_name: String,
    /// Expected workflow type.
    pub workflow_type: InternalSkillWorkflowType,
}

/// Manifest parsed from internal skill descriptors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalSkillManifest {
    /// Unique identifier for the manifest.
    pub manifest_id: String,
    /// Human-readable tool name.
    pub tool_name: String,
    /// Detailed tool description.
    pub description: String,
    /// Type of execution workflow.
    pub workflow_type: InternalSkillWorkflowType,
    /// Target internal binding identifier.
    pub internal_id: String,
    /// Opaque metadata dictionary.
    pub metadata: InternalSkillMetadata,
    /// Tool annotation overrides.
    pub annotations: InternalToolAnnotationOverrides,
    /// Absolute path to the source manifest file.
    pub source_path: PathBuf,
    /// Optional background context for `Qianhuan` rendering.
    #[serde(default)]
    pub qianhuan_background: Option<String>,
    /// Optional serialized flow definition.
    #[serde(default)]
    pub flow_definition: Option<String>,
}

/// Seed payload used when compiling manifests into native aliases.
pub type InternalSkillManifestSeed = InternalSkillManifest;

/// Scan output for internal skill manifests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InternalSkillManifestScan {
    /// Paths to all discovered manifests.
    pub discovered_paths: Vec<PathBuf>,
    /// Successfully parsed and validated manifests.
    pub manifests: Vec<InternalSkillManifest>,
    /// Collection of warnings or errors found during scanning.
    pub issues: Vec<String>,
}

/// Seed payload for native alias compilation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalSkillNativeAliasSeed<Workflow> {
    /// Unique identifier for the manifest.
    pub manifest_id: String,
    /// Human-readable tool name.
    pub tool_name: String,
    /// Detailed tool description.
    pub description: String,
    /// Generic workflow type.
    pub workflow_type: Workflow,
    /// Target internal binding identifier.
    pub internal_id: String,
    /// Opaque metadata dictionary.
    pub metadata: InternalSkillMetadata,
    /// Tool annotation overrides.
    pub annotations: InternalToolAnnotationOverrides,
    /// Absolute path to the source manifest file.
    pub source_path: PathBuf,
}

/// Fully compiled alias spec for an internal skill.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalSkillNativeAliasSpec<Workflow> {
    /// Unique identifier for the manifest.
    pub manifest_id: String,
    /// Human-readable tool name.
    pub tool_name: String,
    /// Detailed tool description.
    pub description: String,
    /// Generic workflow type.
    pub workflow_type: Workflow,
    /// Target internal binding identifier.
    pub internal_id: String,
    /// Opaque metadata dictionary.
    pub metadata: InternalSkillMetadata,
    /// Resolved concrete native tool name.
    pub target_tool_name: String,
    /// Tool annotation overrides.
    pub annotations: InternalToolAnnotationOverrides,
    /// Absolute path to the source manifest file.
    pub source_path: PathBuf,
}

/// Compilation output for a batch of internal aliases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InternalSkillNativeAliasCompilation<Workflow> {
    /// Successfully compiled alias specifications.
    pub compiled_specs: Vec<InternalSkillNativeAliasSpec<Workflow>>,
    /// Collection of compilation errors or warnings.
    pub issues: Vec<String>,
}

/// Mount report for internal aliases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct InternalSkillNativeAliasMountReport<Workflow> {
    /// Root directory for the mount operation.
    pub root: PathBuf,
    /// Paths to all discovered manifests.
    pub discovered_paths: Vec<PathBuf>,
    /// Specs that were successfully mounted.
    pub mounted_specs: Vec<InternalSkillNativeAliasSpec<Workflow>>,
    /// Collection of mount issues.
    pub issues: Vec<String>,
    /// Number of authorized manifests.
    pub authorized_count: usize,
    /// Number of ghost (missing) authorized manifests.
    pub ghost_count: usize,
    /// Number of unauthorized manifests.
    pub unauthorized_count: usize,
}

impl<Workflow> InternalSkillNativeAliasMountReport<Workflow> {
    /// Build a report rooted at the provided directory.
    #[must_use]
    pub fn from_root(root: &std::path::Path) -> Self {
        Self {
            root: root.to_path_buf(),
            discovered_paths: Vec::new(),
            mounted_specs: Vec::new(),
            issues: Vec::new(),
            authorized_count: 0,
            ghost_count: 0,
            unauthorized_count: 0,
        }
    }

    /// Total number of discovered manifest paths.
    #[must_use]
    pub fn discovered_count(&self) -> usize {
        self.discovered_paths.len()
    }

    /// Total number of authorized manifests.
    #[must_use]
    pub const fn authorized_count(&self) -> usize {
        self.authorized_count
    }

    /// Total number of ghost manifests.
    #[must_use]
    pub const fn ghost_count(&self) -> usize {
        self.ghost_count
    }

    /// Total number of unauthorized manifests.
    #[must_use]
    pub const fn unauthorized_count(&self) -> usize {
        self.unauthorized_count
    }

    /// Whether authority drift was detected.
    #[must_use]
    pub const fn has_authority_drift(&self) -> bool {
        self.ghost_count > 0 || self.unauthorized_count > 0
    }

    /// Whether the report indicates a critical failure.
    #[must_use]
    pub const fn is_critically_failed(&self) -> bool {
        self.ghost_count > 0
    }
}

/// Errors produced during internal alias compilation.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum InternalSkillNativeAliasCompileError {
    /// The provided internal binding id is not recognized by the registry.
    #[error("unknown internal binding id: {internal_id}")]
    UnknownBinding {
        /// The offending binding identifier.
        internal_id: String,
    },
}

/// Resolve a validated internal binding id to the concrete native tool name used at runtime.
///
/// # Errors
///
/// Returns an error when the binding id is unknown.
pub fn resolve_internal_skill_binding_target(
    internal_id: &str,
) -> Result<String, InternalSkillNativeAliasCompileError> {
    let bindings = internal_skill_bindings();
    let matched = bindings
        .into_iter()
        .find(|binding| binding.internal_id == internal_id)
        .map(|binding| binding.target_tool_name);
    matched.ok_or_else(|| InternalSkillNativeAliasCompileError::UnknownBinding {
        internal_id: internal_id.to_string(),
    })
}

/// Return the current registry of internal bindings.
#[must_use]
pub fn internal_skill_bindings() -> Vec<InternalSkillBindingDescriptor> {
    vec![
        InternalSkillBindingDescriptor {
            internal_id: "xiuxian.native.zhixing.add".to_string(),
            target_tool_name: "task.add".to_string(),
            workflow_type: InternalSkillWorkflowType::QianjiFlow,
        },
        InternalSkillBindingDescriptor {
            internal_id: "xiuxian.native.zhixing.view".to_string(),
            target_tool_name: "agenda.view".to_string(),
            workflow_type: InternalSkillWorkflowType::NativeDispatch,
        },
        InternalSkillBindingDescriptor {
            internal_id: "xiuxian.native.spider".to_string(),
            target_tool_name: "web.crawl".to_string(),
            workflow_type: InternalSkillWorkflowType::NativeDispatch,
        },
    ]
}

/// Parse a manifest seed payload from raw fields.
#[must_use]
pub fn parse_internal_skill_manifest_seed(
    seed: InternalSkillManifestSeed,
) -> InternalSkillManifest {
    seed
}

/// Apply annotation overrides to one manifest.
#[must_use]
pub fn harden_internal_tool_annotations(
    manifest: InternalSkillManifest,
    overrides: Option<InternalToolAnnotationOverrides>,
) -> InternalSkillManifest {
    if let Some(overrides) = overrides {
        InternalSkillManifest {
            annotations: overrides,
            ..manifest
        }
    } else {
        manifest
    }
}

/// Compile a validated manifest payload into a runtime-ready native alias spec.
#[must_use]
pub fn compile_internal_skill_native_alias<Workflow: Clone>(
    seed: InternalSkillNativeAliasSeed<Workflow>,
) -> Option<InternalSkillNativeAliasSpec<Workflow>> {
    try_compile_internal_skill_native_alias(seed).ok()
}

/// Compile a validated manifest payload into a runtime-ready native alias spec.
///
/// # Errors
///
/// Returns an error when the manifest references an unknown internal runtime binding.
pub fn try_compile_internal_skill_native_alias<Workflow: Clone>(
    seed: InternalSkillNativeAliasSeed<Workflow>,
) -> Result<InternalSkillNativeAliasSpec<Workflow>, InternalSkillNativeAliasCompileError> {
    let target_tool_name = resolve_internal_skill_binding_target(seed.internal_id.as_str())?;
    Ok(InternalSkillNativeAliasSpec {
        manifest_id: seed.manifest_id,
        tool_name: seed.tool_name,
        description: seed.description,
        workflow_type: seed.workflow_type,
        internal_id: seed.internal_id,
        metadata: seed.metadata,
        target_tool_name,
        annotations: seed.annotations,
        source_path: seed.source_path,
    })
}

/// Compile a batch of validated internal manifests into native alias specs.
#[must_use]
pub fn compile_internal_skill_manifest_aliases(
    manifests: Vec<InternalSkillManifest>,
) -> InternalSkillNativeAliasCompilation<InternalSkillWorkflowType> {
    let mut compilation = InternalSkillNativeAliasCompilation {
        compiled_specs: Vec::with_capacity(manifests.len()),
        issues: Vec::new(),
    };
    for manifest in manifests {
        let source_path = manifest.source_path.clone();
        let seed = InternalSkillNativeAliasSeed {
            manifest_id: manifest.manifest_id,
            tool_name: manifest.tool_name,
            description: manifest.description,
            workflow_type: manifest.workflow_type,
            internal_id: manifest.internal_id,
            metadata: manifest.metadata,
            annotations: manifest.annotations,
            source_path: manifest.source_path,
        };
        match try_compile_internal_skill_native_alias(seed) {
            Ok(spec) => compilation.compiled_specs.push(spec),
            Err(error) => compilation
                .issues
                .push(format!("{} -> {error}", source_path.display())),
        }
    }
    compilation
}

/// Build a mount report from compiled specs and discovered paths.
#[must_use]
pub fn resolve_internal_skill_binding_target_from_manifest(
    internal_id: &str,
) -> Result<String, InternalSkillNativeAliasCompileError> {
    resolve_internal_skill_binding_target(internal_id)
}

/// Resolve a validated internal binding target.
#[must_use]
pub fn resolve_internal_skill_binding_target_or_default(
    internal_id: &str,
    default: &str,
) -> String {
    resolve_internal_skill_binding_target(internal_id).unwrap_or_else(|_| default.to_string())
}

/// Build a minimal mount report for compiled specs.
#[must_use]
pub fn build_internal_skill_mount_report<Workflow>(
    discovered_paths: Vec<PathBuf>,
    compiled_specs: Vec<InternalSkillNativeAliasSpec<Workflow>>,
    issues: Vec<String>,
) -> InternalSkillNativeAliasMountReport<Workflow> {
    InternalSkillNativeAliasMountReport {
        root: PathBuf::new(),
        discovered_paths,
        mounted_specs: compiled_specs,
        issues,
        authorized_count: 0,
        ghost_count: 0,
        unauthorized_count: 0,
    }
}
