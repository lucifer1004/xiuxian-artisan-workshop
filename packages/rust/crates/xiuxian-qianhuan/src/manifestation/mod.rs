/// Manifestation manager logic.
pub mod manager;
/// Manifestation render request models.
pub mod request;
/// Template helper logic.
pub mod templates;

pub use manager::ManifestationManager;
pub use request::{
    ManifestationRenderRequest, ManifestationRuntimeContext, ManifestationTemplateTarget,
};
