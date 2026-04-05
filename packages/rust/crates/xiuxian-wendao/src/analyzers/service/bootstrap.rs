use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::registry::PluginRegistry;
use xiuxian_wendao_core::repo_intelligence::builtin_plugin_registrars;

/// Register built-in language analyzers into a fresh plugin registry.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] if a built-in plugin cannot be registered.
pub fn bootstrap_builtin_registry() -> Result<PluginRegistry, RepoIntelligenceError> {
    let mut registry = PluginRegistry::new();
    let mut registrars = builtin_plugin_registrars();
    registrars.sort_by(|left, right| left.plugin_id().cmp(right.plugin_id()));
    for registrar in registrars {
        registrar.register(&mut registry)?;
    }

    Ok(registry)
}

#[cfg(all(test, feature = "modelica"))]
mod tests {
    use super::bootstrap_builtin_registry;

    #[test]
    fn bootstrap_builtin_registry_registers_modelica_plugin() {
        let registry = bootstrap_builtin_registry()
            .unwrap_or_else(|error| panic!("builtin registry bootstrap should succeed: {error}"));

        assert!(
            registry.get("modelica").is_some(),
            "builtin registry should include the external Modelica plugin"
        );
    }
}
