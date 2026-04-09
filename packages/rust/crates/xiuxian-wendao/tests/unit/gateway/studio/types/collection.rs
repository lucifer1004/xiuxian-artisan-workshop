use specta_typescript::{BigIntExportBehavior, Typescript};

use super::studio_type_collection;

#[test]
fn studio_type_collection_exports_generic_plugin_artifact_types_only() {
    let exported = Typescript::new()
        .bigint(BigIntExportBehavior::Number)
        .export(&studio_type_collection())
        .unwrap_or_else(|error| panic!("export studio typescript bindings: {error}"));

    assert!(exported.contains("UiPluginArtifact"));
    assert!(exported.contains("UiPluginLaunchSpec"));
    assert!(!exported.contains("UiCompatDeploymentArtifact"));
    assert!(!exported.contains("UiJuliaDeploymentArtifact"));
}
