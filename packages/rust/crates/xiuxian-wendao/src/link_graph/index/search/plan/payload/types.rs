use crate::link_graph::{LinkGraphPromotedOverlayTelemetry, LinkGraphSearchOptions};

#[derive(Debug, Clone)]
pub(in crate::link_graph::index::search::plan) struct PlannedPayloadBuildContext {
    pub promoted_overlay: Option<LinkGraphPromotedOverlayTelemetry>,
    pub query_vector_override: Option<Vec<f32>>,
}

#[derive(Debug, Clone)]
pub(in crate::link_graph::index::search::plan) struct PlannedPayloadSearchRequest {
    pub query: String,
    pub limit: usize,
    pub base_options: LinkGraphSearchOptions,
    pub include_provisional: Option<bool>,
    pub provisional_limit: Option<usize>,
    pub build_context: PlannedPayloadBuildContext,
}
