use serde::{Deserialize, Serialize};
use xiuxian_wendao_core::artifacts::PluginLaunchSpec;

/// Additive analyzer-owned launch inputs resolved from Julia rerank runtime config.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LinkGraphJuliaAnalyzerServiceDescriptor {
    /// Generic analyzer service mode, usually `stream` or `table`.
    pub service_mode: Option<String>,
    /// Optional path to analyzer-local TOML configuration.
    pub analyzer_config_path: Option<String>,
    /// Optional analyzer strategy override.
    pub analyzer_strategy: Option<String>,
    /// Optional analyzer vector weight.
    pub vector_weight: Option<f64>,
    /// Optional analyzer similarity weight.
    pub similarity_weight: Option<f64>,
}

impl LinkGraphJuliaAnalyzerServiceDescriptor {
    /// Build the generic plugin launch specification using a Julia-owned arg mapping.
    #[must_use]
    pub fn plugin_launch_spec(&self, launcher_path: impl Into<String>) -> PluginLaunchSpec {
        let mut args = Vec::new();

        if let Some(service_mode) = self.service_mode.clone() {
            args.push("--service-mode".to_string());
            args.push(service_mode);
        }
        if let Some(config_path) = self.analyzer_config_path.clone() {
            args.push("--analyzer-config".to_string());
            args.push(config_path);
        }
        if let Some(strategy) = self.analyzer_strategy.clone() {
            args.push("--analyzer-strategy".to_string());
            args.push(strategy);
        }
        if let Some(vector_weight) = self.vector_weight {
            args.push("--vector-weight".to_string());
            args.push(vector_weight.to_string());
        }
        if let Some(similarity_weight) = self.similarity_weight {
            args.push("--similarity-weight".to_string());
            args.push(similarity_weight.to_string());
        }

        PluginLaunchSpec {
            launcher_path: launcher_path.into(),
            args,
        }
    }
}

/// Resolved Julia service launch manifest derived from runtime configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkGraphJuliaAnalyzerLaunchManifest {
    /// Launcher path relative to the repository root.
    pub launcher_path: String,
    /// Ordered analyzer-owned CLI args.
    pub args: Vec<String>,
}

impl From<PluginLaunchSpec> for LinkGraphJuliaAnalyzerLaunchManifest {
    fn from(value: PluginLaunchSpec) -> Self {
        Self {
            launcher_path: value.launcher_path,
            args: value.args,
        }
    }
}

impl From<LinkGraphJuliaAnalyzerLaunchManifest> for PluginLaunchSpec {
    fn from(value: LinkGraphJuliaAnalyzerLaunchManifest) -> Self {
        Self {
            launcher_path: value.launcher_path,
            args: value.args,
        }
    }
}
