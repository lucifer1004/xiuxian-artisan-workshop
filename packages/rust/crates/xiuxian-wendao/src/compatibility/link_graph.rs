/// Compatibility-first runtime-config exports retained at the crate root.
///
/// This module is the preferred public migration path for downstream callers
/// that still need the compat-first runtime-config DTOs and deployment helpers
/// while the crate root continues to re-export them for compatibility.
pub use crate::link_graph::runtime_config::{
    export_link_graph_compat_deployment_artifact_toml,
    LinkGraphCompatAnalyzerLaunchManifest, LinkGraphCompatDeploymentArtifact,
    LinkGraphCompatRerankRuntimeConfig, resolve_link_graph_compat_deployment_artifact,
};

#[cfg(test)]
mod tests {
    use super::{
        export_link_graph_compat_deployment_artifact_toml,
        LinkGraphCompatAnalyzerLaunchManifest, LinkGraphCompatDeploymentArtifact,
        LinkGraphCompatRerankRuntimeConfig, resolve_link_graph_compat_deployment_artifact,
    };

    #[test]
    fn compat_link_graph_crate_root_exports_are_available() {
        let _ = core::mem::size_of::<LinkGraphCompatAnalyzerLaunchManifest>();
        let _ = core::mem::size_of::<LinkGraphCompatDeploymentArtifact>();
        let _ = core::mem::size_of::<LinkGraphCompatRerankRuntimeConfig>();
        let _resolver: fn() -> LinkGraphCompatDeploymentArtifact =
            resolve_link_graph_compat_deployment_artifact;
        let _exporter: fn() -> Result<String, toml::ser::Error> =
            export_link_graph_compat_deployment_artifact_toml;
    }
}
