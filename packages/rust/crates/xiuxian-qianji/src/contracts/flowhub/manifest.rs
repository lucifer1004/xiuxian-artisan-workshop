use serde::{Deserialize, Serialize};

use super::{FlowhubStructureContract, FlowhubValidationRule, TemplateLinkSpec, TemplateUseSpec};

/// Flowhub module metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowhubModuleMetadata {
    /// Stable module name within one Flowhub.
    pub name: String,
    /// Discovery and query metadata.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Stable exported handles declared by a Flowhub module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowhubModuleExports {
    /// Primary entry handle.
    pub entry: String,
    /// Primary completion handle.
    pub ready: String,
}

/// Shared `[template]` composition table used by scenario roots and composite
/// modules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowhubTemplateComposition {
    /// Selected Flowhub modules with explicit aliases.
    #[serde(rename = "use")]
    pub use_entries: Vec<TemplateUseSpec>,
    /// Optional graph links between selected modules or local symbols.
    #[serde(default)]
    pub link: Vec<TemplateLinkSpec>,
}

/// Module-root Flowhub manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowhubModuleManifest {
    /// Planning manifest schema version.
    pub version: u64,
    /// Module metadata.
    pub module: FlowhubModuleMetadata,
    /// Stable exported handles.
    pub exports: FlowhubModuleExports,
    /// Optional child-node filesystem contract for owned subgraphs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract: Option<FlowhubStructureContract>,
    /// Optional internal child-module composition for composite modules.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<FlowhubTemplateComposition>,
    /// Declared validation rules.
    #[serde(default)]
    pub validation: Vec<FlowhubValidationRule>,
}

/// Scenario metadata for Flowhub composition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowhubScenarioPlanning {
    /// Stable scenario name.
    pub name: String,
    /// Scenario tags for discovery and diagnostics.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Backward-compatible alias for the scenario-root `[template]` table.
pub type FlowhubScenarioTemplate = FlowhubTemplateComposition;

/// Scenario-root Flowhub manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowhubScenarioManifest {
    /// Planning manifest schema version.
    pub version: u64,
    /// Scenario metadata.
    pub planning: FlowhubScenarioPlanning,
    /// Selected modules and link declarations.
    pub template: FlowhubTemplateComposition,
}
