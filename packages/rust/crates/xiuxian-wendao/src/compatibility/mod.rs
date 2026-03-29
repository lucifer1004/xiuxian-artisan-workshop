/// Crate-root compatibility namespaces for stable downstream migration paths.
///
/// Downstream callers should use these namespaced exports as the explicit
/// public home for compatibility paths. The flat crate-root Julia-named
/// re-exports and the deprecated `compatibility::julia` shim are retired; the
/// compat-first `link_graph` namespace is now the only remaining crate-root
/// compatibility surface.
///
/// # Examples
///
/// ```rust
/// use xiuxian_wendao::compatibility::link_graph::{
///     LinkGraphCompatDeploymentArtifact, LinkGraphCompatRerankRuntimeConfig,
///     resolve_link_graph_compat_deployment_artifact,
/// };
///
/// let _ = core::mem::size_of::<LinkGraphCompatRerankRuntimeConfig>();
/// let _resolver: fn() -> LinkGraphCompatDeploymentArtifact =
///     resolve_link_graph_compat_deployment_artifact;
/// ```
pub mod link_graph;
