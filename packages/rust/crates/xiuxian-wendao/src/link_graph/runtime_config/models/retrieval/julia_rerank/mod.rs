mod artifact;
mod conversions;
mod launch;
mod runtime;

pub use conversions::build_rerank_provider_binding;
pub use artifact::LinkGraphJuliaDeploymentArtifact;
pub use launch::LinkGraphJuliaAnalyzerLaunchManifest;
pub use runtime::LinkGraphJuliaRerankRuntimeConfig;
pub use xiuxian_wendao_julia::compatibility::link_graph::{
    julia_deployment_artifact_selector, julia_rerank_provider_selector,
};

/// Compatibility-first alias for the Julia deployment-artifact record.
pub type LinkGraphCompatDeploymentArtifact = LinkGraphJuliaDeploymentArtifact;
/// Compatibility-first alias for the Julia analyzer launch manifest.
pub type LinkGraphCompatAnalyzerLaunchManifest = LinkGraphJuliaAnalyzerLaunchManifest;
/// Compatibility-first alias for the Julia rerank runtime record.
pub type LinkGraphCompatRerankRuntimeConfig = LinkGraphJuliaRerankRuntimeConfig;
