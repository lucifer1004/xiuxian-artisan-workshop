use super::errors::RepoIntelligenceError;
use super::registry::PluginRegistry;

/// One plugin-owned builtin registrar that can extend a host registry.
pub struct BuiltinPluginRegistrar {
    plugin_id: &'static str,
    register: fn(&mut PluginRegistry) -> Result<(), RepoIntelligenceError>,
}

impl BuiltinPluginRegistrar {
    /// Create one builtin registrar entry.
    #[must_use]
    pub const fn new(
        plugin_id: &'static str,
        register: fn(&mut PluginRegistry) -> Result<(), RepoIntelligenceError>,
    ) -> Self {
        Self {
            plugin_id,
            register,
        }
    }

    /// Return the stable plugin identifier for this registrar.
    #[must_use]
    pub const fn plugin_id(&self) -> &'static str {
        self.plugin_id
    }

    /// Register this plugin into the provided registry.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when plugin registration fails.
    pub fn register(&self, registry: &mut PluginRegistry) -> Result<(), RepoIntelligenceError> {
        (self.register)(registry)
    }
}

inventory::collect!(BuiltinPluginRegistrar);

/// Collect all plugin-owned builtin registrars linked into the current build.
#[must_use]
pub fn builtin_plugin_registrars() -> Vec<&'static BuiltinPluginRegistrar> {
    inventory::iter::<BuiltinPluginRegistrar>
        .into_iter()
        .collect()
}
