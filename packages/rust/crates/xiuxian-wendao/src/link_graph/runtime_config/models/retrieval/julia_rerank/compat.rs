#[cfg(test)]
use super::{
    LinkGraphJuliaAnalyzerLaunchManifest, LinkGraphJuliaDeploymentArtifact,
    LinkGraphJuliaRerankRuntimeConfig,
};

/// Compatibility-first alias for the Julia deployment-artifact record.
#[cfg(test)]
pub(crate) type LinkGraphCompatDeploymentArtifact = LinkGraphJuliaDeploymentArtifact;
/// Compatibility-first alias for the Julia analyzer launch manifest.
#[cfg(test)]
pub(crate) type LinkGraphCompatAnalyzerLaunchManifest = LinkGraphJuliaAnalyzerLaunchManifest;
/// Compatibility-first alias for the Julia rerank runtime record.
#[cfg(test)]
pub(crate) type LinkGraphCompatRerankRuntimeConfig = LinkGraphJuliaRerankRuntimeConfig;
