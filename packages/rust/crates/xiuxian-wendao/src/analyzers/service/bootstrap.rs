use crate::analyzers::errors::RepoIntelligenceError;
#[cfg(feature = "julia")]
use crate::analyzers::languages::register_julia_plugin;
#[cfg(feature = "modelica")]
use crate::analyzers::languages::register_modelica_plugin;
use crate::analyzers::registry::PluginRegistry;

/// Register built-in language analyzers into a fresh plugin registry.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] if a built-in plugin cannot be registered.
#[allow(clippy::unnecessary_wraps)]
pub fn bootstrap_builtin_registry() -> Result<PluginRegistry, RepoIntelligenceError> {
    #[allow(unused_mut)]
    let mut registry = PluginRegistry::new();

    #[cfg(feature = "julia")]
    {
        register_julia_plugin(&mut registry)?;
    }

    #[cfg(feature = "modelica")]
    {
        register_modelica_plugin(&mut registry)?;
    }

    Ok(registry)
}
