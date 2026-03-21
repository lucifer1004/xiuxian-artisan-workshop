use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Repository-level record produced by Repo Intelligence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepositoryRecord {
    /// Stable repository identifier.
    pub repo_id: String,
    /// Canonical repository display name.
    pub name: String,
    /// Local repository root path used for analysis.
    pub path: String,
    /// Upstream repository URL.
    pub url: Option<String>,
    /// Resolved revision string, when available.
    pub revision: Option<String>,
    /// Parsed project version, when available.
    pub version: Option<String>,
    /// Parsed project UUID, when available.
    pub uuid: Option<String>,
    /// Parsed dependency names, when available.
    pub dependencies: Vec<String>,
}

/// Module or package record discovered within a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ModuleRecord {
    /// Owning repository identifier.
    pub repo_id: String,
    /// Stable module identifier.
    pub module_id: String,
    /// Qualified module name.
    pub qualified_name: String,
    /// Repository-relative source path.
    pub path: String,
}

/// Supported symbol kinds for repository intelligence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RepoSymbolKind {
    /// A function or callable symbol.
    Function,
    /// A type, class, or struct.
    Type,
    /// A module-level constant or value.
    Constant,
    /// A package or module alias/export surface.
    ModuleExport,
    /// A symbol kind not yet normalized by the common core.
    Other,
}

/// Symbol record discovered within a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SymbolRecord {
    /// Owning repository identifier.
    pub repo_id: String,
    /// Stable symbol identifier.
    pub symbol_id: String,
    /// Optional parent module identifier.
    pub module_id: Option<String>,
    /// Display name of the symbol.
    pub name: String,
    /// Qualified symbol name.
    pub qualified_name: String,
    /// Normalized symbol kind.
    pub kind: RepoSymbolKind,
    /// Repository-relative source path.
    pub path: String,
    /// Optional signature string.
    pub signature: Option<String>,
    /// Optional audit status emitted by repository analyzers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_status: Option<String>,
}

/// Example or tutorial seed record discovered within a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExampleRecord {
    /// Owning repository identifier.
    pub repo_id: String,
    /// Stable example identifier.
    pub example_id: String,
    /// Human-readable example title.
    pub title: String,
    /// Repository-relative source path.
    pub path: String,
    /// Optional short description.
    pub summary: Option<String>,
}

/// Documentation record discovered or projected from a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocRecord {
    /// Owning repository identifier.
    pub repo_id: String,
    /// Stable documentation identifier.
    pub doc_id: String,
    /// Human-readable title.
    pub title: String,
    /// Repository-relative source path.
    pub path: String,
    /// Optional documentation format hint.
    pub format: Option<String>,
}

/// Relation kinds for Repo Intelligence mixed graphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RelationKind {
    /// Parent object contains child object.
    Contains,
    /// Module or file declares a symbol.
    Declares,
    /// A symbol or module uses another symbol or module.
    Uses,
    /// A symbol or type implements another interface or concept.
    Implements,
    /// Documentation directly documents an entity.
    Documents,
    /// Example demonstrates a symbol or module.
    ExampleOf,
}

/// Typed relation between two Repo Intelligence objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RelationRecord {
    /// Owning repository identifier.
    pub repo_id: String,
    /// Source object identifier.
    pub source_id: String,
    /// Target object identifier.
    pub target_id: String,
    /// Normalized relation kind.
    pub kind: RelationKind,
}

/// Diagnostic emitted during repository analysis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DiagnosticRecord {
    /// Owning repository identifier.
    pub repo_id: String,
    /// Optional plugin identifier that emitted the diagnostic.
    pub plugin_id: Option<String>,
    /// Repository-relative file path associated with the diagnostic.
    pub path: Option<String>,
    /// Human-readable diagnostic message.
    pub message: String,
}
