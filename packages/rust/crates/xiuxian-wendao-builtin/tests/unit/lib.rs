use crate::bootstrap_builtin_registry;

#[cfg(not(any(feature = "julia", feature = "modelica")))]
#[test]
fn bootstrap_builtin_registry_succeeds_without_feature_plugins() {
    let registry = bootstrap_builtin_registry()
        .unwrap_or_else(|error| panic!("builtin registry bootstrap should succeed: {error}"));

    assert!(
        registry.plugin_ids().is_empty(),
        "default bundle build should not link feature-gated builtin plugins"
    );
}

#[cfg(feature = "julia")]
#[test]
fn bootstrap_builtin_registry_registers_julia_plugin() {
    let registry = bootstrap_builtin_registry()
        .unwrap_or_else(|error| panic!("builtin registry bootstrap should succeed: {error}"));

    assert!(
        registry.get("julia").is_some(),
        "builtin registry should include the external Julia plugin"
    );
}

#[cfg(feature = "modelica")]
#[test]
fn bootstrap_builtin_registry_registers_modelica_plugin() {
    let registry = bootstrap_builtin_registry()
        .unwrap_or_else(|error| panic!("builtin registry bootstrap should succeed: {error}"));

    assert!(
        registry.get("modelica").is_some(),
        "builtin registry should include the external Modelica plugin"
    );
}
