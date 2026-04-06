pub use xiuxian_wendao_builtin::bootstrap_builtin_registry;

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
