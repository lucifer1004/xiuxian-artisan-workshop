use specta::TypeCollection;

use super::config::{UiPluginArtifact, UiPluginLaunchSpec};

/// Build the Studio Specta type collection used by `export_types`.
#[must_use]
pub fn studio_type_collection() -> TypeCollection {
    TypeCollection::default()
        .register::<UiPluginArtifact>()
        .register::<UiPluginLaunchSpec>()
}

#[cfg(test)]
#[path = "../../../../tests/unit/gateway/studio/types/collection.rs"]
mod tests;
