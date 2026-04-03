use arrow::record_batch::RecordBatch;
use xiuxian_wendao_core::repo_intelligence::RepoIntelligenceError;

use super::graph_structural_exchange::{
    GraphStructuralFilterRequestRow, GraphStructuralRerankRequestRow,
    build_graph_structural_rerank_request_batch,
};

/// One query anchor used to align graph-structural request rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStructuralQueryAnchor {
    plane: String,
    value: String,
}

impl GraphStructuralQueryAnchor {
    /// Create one normalized graph-structural query anchor.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when `plane` or `value` is blank after
    /// normalization.
    pub fn new(
        plane: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, RepoIntelligenceError> {
        Ok(Self {
            plane: normalize_non_blank(plane.into(), "query anchor plane")?,
            value: normalize_non_blank(value.into(), "query anchor value")?,
        })
    }

    /// Return the normalized anchor plane.
    #[must_use]
    pub fn plane(&self) -> &str {
        &self.plane
    }

    /// Return the normalized anchor value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

/// Query-scoped context shared by graph-structural request rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStructuralQueryContext {
    query_id: String,
    retrieval_layer: i32,
    query_max_layers: i32,
    anchors: Vec<GraphStructuralQueryAnchor>,
    edge_constraint_kinds: Vec<String>,
}

impl GraphStructuralQueryContext {
    /// Create normalized query context for graph-structural request rows.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when the query id is blank, the layer
    /// bounds are invalid, the anchor list is empty, or any edge-constraint
    /// value is blank.
    pub fn new(
        query_id: impl Into<String>,
        retrieval_layer: i32,
        query_max_layers: i32,
        anchors: Vec<GraphStructuralQueryAnchor>,
        edge_constraint_kinds: Vec<String>,
    ) -> Result<Self, RepoIntelligenceError> {
        if retrieval_layer < 0 {
            return Err(graph_structural_projection_error(format!(
                "retrieval layer must be non-negative; found {retrieval_layer}"
            )));
        }
        if query_max_layers < 1 {
            return Err(graph_structural_projection_error(format!(
                "query max layers must be at least 1; found {query_max_layers}"
            )));
        }
        if anchors.is_empty() {
            return Err(graph_structural_projection_error(
                "at least one query anchor is required",
            ));
        }
        Ok(Self {
            query_id: normalize_non_blank(query_id.into(), "query id")?,
            retrieval_layer,
            query_max_layers,
            anchors,
            edge_constraint_kinds: normalize_string_list(
                edge_constraint_kinds,
                "edge constraint kinds",
                true,
            )?,
        })
    }

    /// Return the normalized query id.
    #[must_use]
    pub fn query_id(&self) -> &str {
        &self.query_id
    }

    /// Return the retrieval layer for this query context.
    #[must_use]
    pub fn retrieval_layer(&self) -> i32 {
        self.retrieval_layer
    }

    /// Return the maximum retrieval depth for this query context.
    #[must_use]
    pub fn query_max_layers(&self) -> i32 {
        self.query_max_layers
    }

    /// Return the normalized anchor list.
    #[must_use]
    pub fn anchors(&self) -> &[GraphStructuralQueryAnchor] {
        &self.anchors
    }

    /// Return the normalized edge-constraint kinds.
    #[must_use]
    pub fn edge_constraint_kinds(&self) -> &[String] {
        &self.edge_constraint_kinds
    }
}

/// Bounded candidate subgraph selected for structural evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStructuralCandidateSubgraph {
    candidate_id: String,
    node_ids: Vec<String>,
    edge_kinds: Vec<String>,
}

impl GraphStructuralCandidateSubgraph {
    /// Create one normalized graph-structural candidate subgraph.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when the candidate id is blank, the
    /// node list is empty, or any node or edge-kind value is blank.
    pub fn new(
        candidate_id: impl Into<String>,
        node_ids: Vec<String>,
        edge_kinds: Vec<String>,
    ) -> Result<Self, RepoIntelligenceError> {
        Ok(Self {
            candidate_id: normalize_non_blank(candidate_id.into(), "candidate id")?,
            node_ids: normalize_string_list(node_ids, "candidate node ids", false)?,
            edge_kinds: normalize_string_list(edge_kinds, "candidate edge kinds", true)?,
        })
    }

    /// Return the normalized candidate id.
    #[must_use]
    pub fn candidate_id(&self) -> &str {
        &self.candidate_id
    }

    /// Return the normalized candidate node ids.
    #[must_use]
    pub fn node_ids(&self) -> &[String] {
        &self.node_ids
    }

    /// Return the normalized candidate edge kinds.
    #[must_use]
    pub fn edge_kinds(&self) -> &[String] {
        &self.edge_kinds
    }
}

/// Per-plane scores that feed structural rerank evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphStructuralRerankSignals {
    semantic: f64,
    dependency: f64,
    keyword: f64,
    tag: f64,
}

impl GraphStructuralRerankSignals {
    /// Create one normalized set of structural rerank plane scores.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when any score is negative or not
    /// finite.
    pub fn new(
        semantic_score: f64,
        dependency_score: f64,
        keyword_score: f64,
        tag_score: f64,
    ) -> Result<Self, RepoIntelligenceError> {
        Ok(Self {
            semantic: normalize_non_negative_score(semantic_score, "semantic score")?,
            dependency: normalize_non_negative_score(dependency_score, "dependency score")?,
            keyword: normalize_non_negative_score(keyword_score, "keyword score")?,
            tag: normalize_non_negative_score(tag_score, "tag score")?,
        })
    }

    /// Return the normalized semantic score.
    #[must_use]
    pub fn semantic_score(&self) -> f64 {
        self.semantic
    }

    /// Return the normalized dependency score.
    #[must_use]
    pub fn dependency_score(&self) -> f64 {
        self.dependency
    }

    /// Return the normalized keyword score.
    #[must_use]
    pub fn keyword_score(&self) -> f64 {
        self.keyword
    }

    /// Return the normalized tag score.
    #[must_use]
    pub fn tag_score(&self) -> f64 {
        self.tag
    }
}

/// Raw query inputs for the keyword-or-tag graph-structural helper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStructuralKeywordTagQueryInputs {
    query_id: String,
    retrieval_layer: i32,
    query_max_layers: i32,
    keyword_anchors: Vec<String>,
    tag_anchors: Vec<String>,
    edge_constraint_kinds: Vec<String>,
}

impl GraphStructuralKeywordTagQueryInputs {
    /// Store one keyword-or-tag query input bundle for later normalization.
    #[must_use]
    pub fn new(
        query_id: impl Into<String>,
        retrieval_layer: i32,
        query_max_layers: i32,
        keyword_anchors: Vec<String>,
        tag_anchors: Vec<String>,
        edge_constraint_kinds: Vec<String>,
    ) -> Self {
        Self {
            query_id: query_id.into(),
            retrieval_layer,
            query_max_layers,
            keyword_anchors,
            tag_anchors,
            edge_constraint_kinds,
        }
    }
}

/// Raw pair-candidate inputs for the keyword-or-tag graph-structural helper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStructuralPairCandidateInputs {
    left_id: String,
    right_id: String,
    edge_kinds: Vec<String>,
}

impl GraphStructuralPairCandidateInputs {
    /// Store one pair-candidate input bundle for later normalization.
    #[must_use]
    pub fn new(
        left_id: impl Into<String>,
        right_id: impl Into<String>,
        edge_kinds: Vec<String>,
    ) -> Self {
        Self {
            left_id: left_id.into(),
            right_id: right_id.into(),
            edge_kinds,
        }
    }
}

/// Build one pair-candidate input bundle from raw pair ids and edge kinds.
///
/// This keeps host consumers on the plugin-owned staging seam instead of
/// manually constructing the pair-input DTO layer.
#[must_use]
pub fn build_graph_structural_pair_candidate_inputs(
    left_id: impl Into<String>,
    right_id: impl Into<String>,
    edge_kinds: Vec<String>,
) -> GraphStructuralPairCandidateInputs {
    GraphStructuralPairCandidateInputs::new(left_id, right_id, edge_kinds)
}

/// Raw node-metadata inputs for the graph-structural projection helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStructuralNodeMetadataInputs {
    tags: Vec<String>,
}

impl GraphStructuralNodeMetadataInputs {
    /// Store one node-metadata input bundle for later normalization.
    #[must_use]
    pub fn new(tags: Vec<String>) -> Self {
        Self { tags }
    }
}

/// Raw keyword-overlap inputs that combine query, metadata, and pair data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStructuralKeywordOverlapPairInputs {
    query_inputs: GraphStructuralKeywordTagQueryInputs,
    left_metadata: GraphStructuralNodeMetadataInputs,
    right_metadata: GraphStructuralNodeMetadataInputs,
    pair_inputs: GraphStructuralPairCandidateInputs,
}

impl GraphStructuralKeywordOverlapPairInputs {
    /// Store one keyword-overlap input bundle for later normalization.
    #[must_use]
    pub fn new(
        query_inputs: GraphStructuralKeywordTagQueryInputs,
        left_metadata: GraphStructuralNodeMetadataInputs,
        right_metadata: GraphStructuralNodeMetadataInputs,
        pair_inputs: GraphStructuralPairCandidateInputs,
    ) -> Self {
        Self {
            query_inputs,
            left_metadata,
            right_metadata,
            pair_inputs,
        }
    }
}

/// Raw scored inputs for one metadata-aware rerank request.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphStructuralKeywordOverlapPairRerankInputs {
    metadata_inputs: GraphStructuralKeywordOverlapPairInputs,
    semantic_score: f64,
    dependency_score: f64,
    keyword_match: bool,
}

impl GraphStructuralKeywordOverlapPairRerankInputs {
    /// Store one metadata-aware rerank input bundle for later normalization.
    #[must_use]
    pub fn new(
        metadata_inputs: GraphStructuralKeywordOverlapPairInputs,
        semantic_score: f64,
        dependency_score: f64,
        keyword_match: bool,
    ) -> Self {
        Self {
            metadata_inputs,
            semantic_score,
            dependency_score,
            keyword_match,
        }
    }
}

/// Higher-level metadata-aware request inputs for one keyword-overlap rerank request.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphStructuralKeywordOverlapPairRequestInputs {
    metadata_inputs: GraphStructuralKeywordOverlapPairInputs,
    semantic_score: f64,
    dependency_score: f64,
    keyword_match: bool,
}

impl GraphStructuralKeywordOverlapPairRequestInputs {
    /// Store one higher-level keyword-overlap request input bundle.
    #[must_use]
    pub fn new(
        query_inputs: GraphStructuralKeywordTagQueryInputs,
        left_metadata: GraphStructuralNodeMetadataInputs,
        right_metadata: GraphStructuralNodeMetadataInputs,
        pair_inputs: GraphStructuralPairCandidateInputs,
        semantic_score: f64,
        dependency_score: f64,
        keyword_match: bool,
    ) -> Self {
        Self {
            metadata_inputs: GraphStructuralKeywordOverlapPairInputs::new(
                query_inputs,
                left_metadata,
                right_metadata,
                pair_inputs,
            ),
            semantic_score,
            dependency_score,
            keyword_match,
        }
    }
}

/// Shared query inputs reused across keyword-overlap pair requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStructuralKeywordOverlapQueryInputs {
    query_id: String,
    retrieval_layer: i32,
    query_max_layers: i32,
    keyword_anchors: Vec<String>,
    edge_constraint_kinds: Vec<String>,
}

impl GraphStructuralKeywordOverlapQueryInputs {
    /// Store one shared keyword-overlap query input bundle.
    #[must_use]
    pub fn new(
        query_id: impl Into<String>,
        retrieval_layer: i32,
        query_max_layers: i32,
        keyword_anchors: Vec<String>,
        edge_constraint_kinds: Vec<String>,
    ) -> Self {
        Self {
            query_id: query_id.into(),
            retrieval_layer,
            query_max_layers,
            keyword_anchors,
            edge_constraint_kinds,
        }
    }
}

/// Build one shared keyword-overlap query input bundle from raw query fields.
///
/// This keeps host consumers on the plugin-owned staging seam instead of
/// manually constructing the shared-query DTO layer.
#[must_use]
pub fn build_graph_structural_keyword_overlap_query_inputs(
    query_id: impl Into<String>,
    retrieval_layer: i32,
    query_max_layers: i32,
    keyword_anchors: Vec<String>,
    edge_constraint_kinds: Vec<String>,
) -> GraphStructuralKeywordOverlapQueryInputs {
    GraphStructuralKeywordOverlapQueryInputs::new(
        query_id,
        retrieval_layer,
        query_max_layers,
        keyword_anchors,
        edge_constraint_kinds,
    )
}

/// Raw per-candidate inputs reused with one shared keyword-overlap query.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphStructuralKeywordOverlapRawCandidateInputs {
    metadata_inputs: GraphStructuralKeywordOverlapCandidateMetadataInputs,
    semantic_score: f64,
    dependency_score: f64,
    keyword_match: bool,
}

impl GraphStructuralKeywordOverlapRawCandidateInputs {
    /// Store one raw keyword-overlap candidate input bundle.
    #[must_use]
    pub fn new(
        metadata_inputs: GraphStructuralKeywordOverlapCandidateMetadataInputs,
        semantic_score: f64,
        dependency_score: f64,
        keyword_match: bool,
    ) -> Self {
        Self {
            metadata_inputs,
            semantic_score,
            dependency_score,
            keyword_match,
        }
    }
}

/// Build one raw keyword-overlap candidate bundle from pair metadata and
/// retrieval scores.
///
/// This keeps host consumers on a plugin-owned raw staging seam instead of
/// manually assembling per-candidate raw DTOs inline.
#[must_use]
pub fn build_graph_structural_keyword_overlap_raw_candidate_inputs(
    metadata_inputs: GraphStructuralKeywordOverlapCandidateMetadataInputs,
    semantic_score: f64,
    dependency_score: f64,
    keyword_match: bool,
) -> GraphStructuralKeywordOverlapRawCandidateInputs {
    GraphStructuralKeywordOverlapRawCandidateInputs::new(
        metadata_inputs,
        semantic_score,
        dependency_score,
        keyword_match,
    )
}

/// Per-candidate normalized inputs reused with one shared keyword-overlap query.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphStructuralKeywordOverlapCandidateInputs {
    left_metadata: GraphStructuralNodeMetadataInputs,
    right_metadata: GraphStructuralNodeMetadataInputs,
    pair_inputs: GraphStructuralPairCandidateInputs,
    semantic_score: f64,
    dependency_score: f64,
    keyword_match: bool,
}

impl GraphStructuralKeywordOverlapCandidateInputs {
    /// Store one keyword-overlap candidate input bundle.
    #[must_use]
    pub fn new(
        left_metadata: GraphStructuralNodeMetadataInputs,
        right_metadata: GraphStructuralNodeMetadataInputs,
        pair_inputs: GraphStructuralPairCandidateInputs,
        semantic_score: f64,
        dependency_score: f64,
        keyword_match: bool,
    ) -> Self {
        Self {
            left_metadata,
            right_metadata,
            pair_inputs,
            semantic_score,
            dependency_score,
            keyword_match,
        }
    }
}

/// Raw metadata inputs for one keyword-overlap candidate before score attachment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStructuralKeywordOverlapCandidateMetadataInputs {
    left_metadata: GraphStructuralNodeMetadataInputs,
    right_metadata: GraphStructuralNodeMetadataInputs,
    pair_inputs: GraphStructuralPairCandidateInputs,
}

impl GraphStructuralKeywordOverlapCandidateMetadataInputs {
    /// Store one keyword-overlap candidate metadata bundle.
    #[must_use]
    pub fn new(
        left_metadata: GraphStructuralNodeMetadataInputs,
        right_metadata: GraphStructuralNodeMetadataInputs,
        pair_inputs: GraphStructuralPairCandidateInputs,
    ) -> Self {
        Self {
            left_metadata,
            right_metadata,
            pair_inputs,
        }
    }
}

/// Build one keyword-overlap candidate input bundle from raw pair metadata.
///
/// This keeps host consumers on the plugin-owned staging seam instead of
/// manually assembling node-metadata and pair-candidate DTOs.
#[must_use]
pub fn build_graph_structural_keyword_overlap_candidate_inputs(
    metadata_inputs: GraphStructuralKeywordOverlapCandidateMetadataInputs,
    semantic_score: f64,
    dependency_score: f64,
    keyword_match: bool,
) -> GraphStructuralKeywordOverlapCandidateInputs {
    let GraphStructuralKeywordOverlapCandidateMetadataInputs {
        left_metadata,
        right_metadata,
        pair_inputs,
    } = metadata_inputs;
    GraphStructuralKeywordOverlapCandidateInputs::new(
        left_metadata,
        right_metadata,
        pair_inputs,
        semantic_score,
        dependency_score,
        keyword_match,
    )
}

/// Build one keyword-overlap candidate metadata bundle from raw pair ids, edge
/// kinds, and node tags.
///
/// This keeps host consumers on the plugin-owned staging seam instead of
/// manually assembling node-metadata and pair-candidate DTOs.
#[must_use]
pub fn build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
    left_id: impl Into<String>,
    right_id: impl Into<String>,
    edge_kinds: Vec<String>,
    left_tags: Vec<String>,
    right_tags: Vec<String>,
) -> GraphStructuralKeywordOverlapCandidateMetadataInputs {
    GraphStructuralKeywordOverlapCandidateMetadataInputs::new(
        GraphStructuralNodeMetadataInputs::new(left_tags),
        GraphStructuralNodeMetadataInputs::new(right_tags),
        build_graph_structural_pair_candidate_inputs(left_id, right_id, edge_kinds),
    )
}

/// Build one keyword-overlap candidate input bundle from one staged raw
/// candidate bundle.
///
/// This preserves the narrower plugin-owned seam for callers that already hold
/// the raw candidate bundle.
#[must_use]
pub fn build_graph_structural_keyword_overlap_pair_candidate_inputs_from_raw(
    raw_candidate_inputs: GraphStructuralKeywordOverlapRawCandidateInputs,
) -> GraphStructuralKeywordOverlapCandidateInputs {
    let GraphStructuralKeywordOverlapRawCandidateInputs {
        metadata_inputs,
        semantic_score,
        dependency_score,
        keyword_match,
    } = raw_candidate_inputs;
    build_graph_structural_keyword_overlap_pair_candidate_inputs(
        metadata_inputs,
        semantic_score,
        dependency_score,
        keyword_match,
    )
}

/// Build one keyword-overlap candidate input bundle from staged pair metadata
/// and rerank scores.
///
/// This preserves the narrower plugin-owned seam for callers that already hold
/// the metadata bundle.
#[must_use]
pub fn build_graph_structural_keyword_overlap_pair_candidate_inputs(
    metadata_inputs: GraphStructuralKeywordOverlapCandidateMetadataInputs,
    semantic_score: f64,
    dependency_score: f64,
    keyword_match: bool,
) -> GraphStructuralKeywordOverlapCandidateInputs {
    build_graph_structural_keyword_overlap_candidate_inputs(
        metadata_inputs,
        semantic_score,
        dependency_score,
        keyword_match,
    )
}

/// Build one query context from keyword and tag anchor values.
///
/// Keyword anchors are emitted first, followed by tag anchors.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the query id is blank, the layer
/// bounds are invalid, both anchor lists are empty, or any anchor or
/// edge-constraint value is blank.
pub fn build_graph_structural_keyword_tag_query_context(
    query_id: impl Into<String>,
    retrieval_layer: i32,
    query_max_layers: i32,
    keyword_anchors: Vec<String>,
    tag_anchors: Vec<String>,
    edge_constraint_kinds: Vec<String>,
) -> Result<GraphStructuralQueryContext, RepoIntelligenceError> {
    let mut anchors = Vec::with_capacity(keyword_anchors.len() + tag_anchors.len());
    for keyword in keyword_anchors {
        anchors.push(GraphStructuralQueryAnchor::new("keyword", keyword)?);
    }
    for tag in tag_anchors {
        anchors.push(GraphStructuralQueryAnchor::new("tag", tag)?);
    }
    GraphStructuralQueryContext::new(
        query_id,
        retrieval_layer,
        query_max_layers,
        anchors,
        edge_constraint_kinds,
    )
}

/// Build one rerank-signal set from semantic scores plus binary keyword or tag matches.
///
/// `keyword_match` and `tag_match` are normalized to `1.0` when true and
/// `0.0` when false.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when `semantic_score` or
/// `dependency_score` is negative or not finite.
pub fn build_graph_structural_keyword_tag_rerank_signals(
    semantic_score: f64,
    dependency_score: f64,
    keyword_match: bool,
    tag_match: bool,
) -> Result<GraphStructuralRerankSignals, RepoIntelligenceError> {
    GraphStructuralRerankSignals::new(
        semantic_score,
        dependency_score,
        binary_plane_score(keyword_match),
        binary_plane_score(tag_match),
    )
}

/// Constraint settings that feed structural filter evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphStructuralFilterConstraint {
    constraint_kind: String,
    required_boundary_size: i32,
}

impl GraphStructuralFilterConstraint {
    /// Create one normalized structural filter constraint.
    ///
    /// # Errors
    ///
    /// Returns [`RepoIntelligenceError`] when the constraint kind is blank or
    /// the required boundary size is negative.
    pub fn new(
        constraint_kind: impl Into<String>,
        required_boundary_size: i32,
    ) -> Result<Self, RepoIntelligenceError> {
        if required_boundary_size < 0 {
            return Err(graph_structural_projection_error(format!(
                "required boundary size must be non-negative; found {required_boundary_size}"
            )));
        }
        Ok(Self {
            constraint_kind: normalize_non_blank(constraint_kind.into(), "constraint kind")?,
            required_boundary_size,
        })
    }

    /// Return the normalized constraint kind.
    #[must_use]
    pub fn constraint_kind(&self) -> &str {
        &self.constraint_kind
    }

    /// Return the required boundary size.
    #[must_use]
    pub fn required_boundary_size(&self) -> i32 {
        self.required_boundary_size
    }
}

/// Build one staged structural-rerank request row from normalized semantic DTOs.
#[must_use]
pub fn build_graph_structural_rerank_request_row(
    query: &GraphStructuralQueryContext,
    candidate: &GraphStructuralCandidateSubgraph,
    signals: &GraphStructuralRerankSignals,
) -> GraphStructuralRerankRequestRow {
    GraphStructuralRerankRequestRow {
        query_id: query.query_id.clone(),
        candidate_id: candidate.candidate_id.clone(),
        retrieval_layer: query.retrieval_layer,
        query_max_layers: query.query_max_layers,
        semantic_score: signals.semantic,
        dependency_score: signals.dependency,
        keyword_score: signals.keyword,
        tag_score: signals.tag,
        anchor_planes: query
            .anchors
            .iter()
            .map(|anchor| anchor.plane.clone())
            .collect(),
        anchor_values: query
            .anchors
            .iter()
            .map(|anchor| anchor.value.clone())
            .collect(),
        edge_constraint_kinds: query.edge_constraint_kinds.clone(),
        candidate_node_ids: candidate.node_ids.clone(),
        candidate_edge_kinds: candidate.edge_kinds.clone(),
    }
}

/// Build one staged constraint-filter request row from normalized semantic DTOs.
#[must_use]
pub fn build_graph_structural_filter_request_row(
    query: &GraphStructuralQueryContext,
    candidate: &GraphStructuralCandidateSubgraph,
    constraint: &GraphStructuralFilterConstraint,
) -> GraphStructuralFilterRequestRow {
    GraphStructuralFilterRequestRow {
        query_id: query.query_id.clone(),
        candidate_id: candidate.candidate_id.clone(),
        retrieval_layer: query.retrieval_layer,
        query_max_layers: query.query_max_layers,
        constraint_kind: constraint.constraint_kind.clone(),
        required_boundary_size: constraint.required_boundary_size,
        anchor_planes: query
            .anchors
            .iter()
            .map(|anchor| anchor.plane.clone())
            .collect(),
        anchor_values: query
            .anchors
            .iter()
            .map(|anchor| anchor.value.clone())
            .collect(),
        edge_constraint_kinds: query.edge_constraint_kinds.clone(),
        candidate_node_ids: candidate.node_ids.clone(),
        candidate_edge_kinds: candidate.edge_kinds.clone(),
    }
}

/// Build the stable candidate id used for one two-node graph-structural pair.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when either endpoint id is blank after
/// normalization or when both endpoints resolve to the same id.
pub fn graph_structural_pair_candidate_id(
    left_id: impl Into<String>,
    right_id: impl Into<String>,
) -> Result<String, RepoIntelligenceError> {
    let (left_id, right_id) = normalize_pair_endpoint_ids(left_id.into(), right_id.into())?;
    Ok(stable_pair_candidate_id(&left_id, &right_id))
}

/// Build one normalized candidate subgraph from a two-node graph pair.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when either endpoint id is blank after
/// normalization, both endpoints resolve to the same id, or any edge kind is
/// blank.
pub fn build_graph_structural_pair_candidate_subgraph(
    left_id: impl Into<String>,
    right_id: impl Into<String>,
    edge_kinds: Vec<String>,
) -> Result<GraphStructuralCandidateSubgraph, RepoIntelligenceError> {
    let (left_id, right_id) = normalize_pair_endpoint_ids(left_id.into(), right_id.into())?;
    GraphStructuralCandidateSubgraph::new(
        stable_pair_candidate_id(&left_id, &right_id),
        vec![left_id, right_id],
        edge_kinds,
    )
}

/// Build one staged structural-rerank request row from a two-node graph pair.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when either endpoint id is blank after
/// normalization, both endpoints resolve to the same id, or any edge kind is
/// blank.
pub fn build_graph_structural_pair_rerank_request_row(
    query: &GraphStructuralQueryContext,
    left_id: impl Into<String>,
    right_id: impl Into<String>,
    edge_kinds: Vec<String>,
    signals: &GraphStructuralRerankSignals,
) -> Result<GraphStructuralRerankRequestRow, RepoIntelligenceError> {
    let candidate = build_graph_structural_pair_candidate_subgraph(left_id, right_id, edge_kinds)?;
    Ok(build_graph_structural_rerank_request_row(
        query, &candidate, signals,
    ))
}

/// Build one staged constraint-filter request row from a two-node graph pair.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when either endpoint id is blank after
/// normalization, both endpoints resolve to the same id, or any edge kind is
/// blank.
pub fn build_graph_structural_pair_filter_request_row(
    query: &GraphStructuralQueryContext,
    left_id: impl Into<String>,
    right_id: impl Into<String>,
    edge_kinds: Vec<String>,
    constraint: &GraphStructuralFilterConstraint,
) -> Result<GraphStructuralFilterRequestRow, RepoIntelligenceError> {
    let candidate = build_graph_structural_pair_candidate_subgraph(left_id, right_id, edge_kinds)?;
    Ok(build_graph_structural_filter_request_row(
        query, &candidate, constraint,
    ))
}

/// Build one staged structural-rerank request row from keyword-or-tag query inputs
/// plus one two-node graph pair.
///
/// This convenience helper keeps the host on a thin consumption seam by
/// composing the Julia-owned keyword-or-tag query builder, binary rerank-signal
/// builder, and pair-rerank request-row projection.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when any query, anchor, edge-constraint,
/// endpoint, edge-kind, or score input fails the underlying Julia-owned
/// normalization rules.
pub fn build_graph_structural_keyword_tag_pair_rerank_request_row(
    query_inputs: GraphStructuralKeywordTagQueryInputs,
    pair_inputs: GraphStructuralPairCandidateInputs,
    semantic_score: f64,
    dependency_score: f64,
    keyword_match: bool,
    tag_match: bool,
) -> Result<GraphStructuralRerankRequestRow, RepoIntelligenceError> {
    let query = build_graph_structural_keyword_tag_query_context(
        query_inputs.query_id,
        query_inputs.retrieval_layer,
        query_inputs.query_max_layers,
        query_inputs.keyword_anchors,
        query_inputs.tag_anchors,
        query_inputs.edge_constraint_kinds,
    )?;
    let signals = build_graph_structural_keyword_tag_rerank_signals(
        semantic_score,
        dependency_score,
        keyword_match,
        tag_match,
    )?;
    build_graph_structural_pair_rerank_request_row(
        &query,
        pair_inputs.left_id,
        pair_inputs.right_id,
        pair_inputs.edge_kinds,
        &signals,
    )
}

/// Return normalized shared tags between the left and right metadata slices.
///
/// The output preserves normalized left-tag order and removes duplicates.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when any tag value is blank after
/// normalization.
pub fn graph_structural_shared_tag_anchors(
    left_tags: Vec<String>,
    right_tags: Vec<String>,
) -> Result<Vec<String>, RepoIntelligenceError> {
    let left_tags = normalize_string_list(left_tags, "left tags", true)?;
    let right_tags = normalize_string_list(right_tags, "right tags", true)?;
    let right_set: std::collections::HashSet<String> = right_tags.into_iter().collect();
    let mut seen = std::collections::HashSet::new();
    let mut shared = Vec::new();
    for tag in left_tags {
        if right_set.contains(&tag) && seen.insert(tag.clone()) {
            shared.push(tag);
        }
    }
    Ok(shared)
}

/// Build one staged structural-rerank request row from keyword anchors,
/// raw left or right tag metadata, and one two-node graph pair.
///
/// Shared tag anchors are derived inside the Julia-owned helper layer, and the
/// tag-score signal is inferred from whether any shared tags remain.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when any query, tag, edge-constraint,
/// endpoint, edge-kind, or score input fails the underlying Julia-owned
/// normalization rules.
pub fn build_graph_structural_keyword_overlap_pair_rerank_request_row(
    mut query_inputs: GraphStructuralKeywordTagQueryInputs,
    left_tags: Vec<String>,
    right_tags: Vec<String>,
    pair_inputs: GraphStructuralPairCandidateInputs,
    semantic_score: f64,
    dependency_score: f64,
    keyword_match: bool,
) -> Result<GraphStructuralRerankRequestRow, RepoIntelligenceError> {
    query_inputs.tag_anchors = graph_structural_shared_tag_anchors(left_tags, right_tags)?;
    let tag_match = !query_inputs.tag_anchors.is_empty();
    build_graph_structural_keyword_tag_pair_rerank_request_row(
        query_inputs,
        pair_inputs,
        semantic_score,
        dependency_score,
        keyword_match,
        tag_match,
    )
}

/// Build one staged structural-rerank request row from a plugin-owned
/// metadata-and-pair input bundle.
///
/// This convenience helper keeps the host on a thinner Julia-owned seam by
/// deferring both shared-tag extraction and pair-row assembly to the plugin
/// layer.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when any query, metadata, edge-constraint,
/// endpoint, edge-kind, or score input fails the underlying Julia-owned
/// normalization rules.
pub fn build_graph_structural_keyword_overlap_pair_rerank_request_row_from_metadata(
    inputs: GraphStructuralKeywordOverlapPairInputs,
    semantic_score: f64,
    dependency_score: f64,
    keyword_match: bool,
) -> Result<GraphStructuralRerankRequestRow, RepoIntelligenceError> {
    build_graph_structural_keyword_overlap_pair_rerank_request_row(
        inputs.query_inputs,
        inputs.left_metadata.tags,
        inputs.right_metadata.tags,
        inputs.pair_inputs,
        semantic_score,
        dependency_score,
        keyword_match,
    )
}

/// Build one staged structural-rerank request batch from plugin-owned
/// metadata-aware rerank input bundles.
///
/// This convenience helper keeps the host on the thinnest currently available
/// Julia-owned seam by composing metadata-aware row projection and Arrow batch
/// materialization inside the plugin crate.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when any metadata-aware rerank input fails
/// the underlying Julia-owned normalization rules or when the final Arrow batch
/// fails staged contract validation.
pub fn build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_metadata(
    inputs: &[GraphStructuralKeywordOverlapPairRerankInputs],
) -> Result<RecordBatch, RepoIntelligenceError> {
    let rows = inputs
        .iter()
        .cloned()
        .map(|input| {
            build_graph_structural_keyword_overlap_pair_rerank_request_row_from_metadata(
                input.metadata_inputs,
                input.semantic_score,
                input.dependency_score,
                input.keyword_match,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    build_graph_structural_rerank_request_batch(&rows)
}

/// Build one staged structural-rerank request batch from higher-level
/// keyword-overlap candidate inputs.
///
/// This helper keeps host consumers on a thinner Julia-owned seam by composing
/// query-input, metadata-input, pair-input, and metadata-aware batch assembly
/// inside the plugin crate.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when any higher-level candidate input
/// fails the underlying Julia-owned normalization rules or when the final
/// Arrow batch fails staged contract validation.
pub fn build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_inputs(
    inputs: &[GraphStructuralKeywordOverlapPairRequestInputs],
) -> Result<RecordBatch, RepoIntelligenceError> {
    let metadata_inputs = inputs
        .iter()
        .cloned()
        .map(|input| {
            GraphStructuralKeywordOverlapPairRerankInputs::new(
                input.metadata_inputs,
                input.semantic_score,
                input.dependency_score,
                input.keyword_match,
            )
        })
        .collect::<Vec<_>>();
    build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_metadata(&metadata_inputs)
}

/// Build one higher-level keyword-overlap request input from a shared query
/// bundle and one candidate bundle.
#[must_use]
pub fn build_graph_structural_keyword_overlap_pair_request_input(
    query: &GraphStructuralKeywordOverlapQueryInputs,
    candidate: GraphStructuralKeywordOverlapCandidateInputs,
) -> GraphStructuralKeywordOverlapPairRequestInputs {
    let query_inputs = GraphStructuralKeywordTagQueryInputs::new(
        query.query_id.clone(),
        query.retrieval_layer,
        query.query_max_layers,
        query.keyword_anchors.clone(),
        Vec::new(),
        query.edge_constraint_kinds.clone(),
    );

    GraphStructuralKeywordOverlapPairRequestInputs::new(
        query_inputs,
        candidate.left_metadata,
        candidate.right_metadata,
        candidate.pair_inputs,
        candidate.semantic_score,
        candidate.dependency_score,
        candidate.keyword_match,
    )
}

/// Build one staged structural-rerank request batch from one shared
/// keyword-overlap query bundle plus per-candidate inputs.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when any derived request input fails the
/// underlying Julia-owned normalization rules or when the final Arrow batch
/// fails staged contract validation.
pub fn build_graph_structural_keyword_overlap_pair_rerank_request_batch(
    query: &GraphStructuralKeywordOverlapQueryInputs,
    candidates: &[GraphStructuralKeywordOverlapCandidateInputs],
) -> Result<RecordBatch, RepoIntelligenceError> {
    let request_inputs = candidates
        .iter()
        .cloned()
        .map(|candidate| {
            build_graph_structural_keyword_overlap_pair_request_input(query, candidate)
        })
        .collect::<Vec<_>>();
    build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_inputs(&request_inputs)
}

/// Build one staged structural-rerank request batch from one shared
/// keyword-overlap query bundle plus raw per-candidate inputs.
///
/// This helper keeps host consumers on a thinner Julia-owned seam by
/// composing raw candidate normalization, higher-level request-input
/// projection, and Arrow batch materialization inside the plugin crate.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when any raw candidate input fails the
/// underlying Julia-owned normalization rules or when the final Arrow batch
/// fails staged contract validation.
pub fn build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_raw_candidates(
    query: &GraphStructuralKeywordOverlapQueryInputs,
    candidates: &[GraphStructuralKeywordOverlapRawCandidateInputs],
) -> Result<RecordBatch, RepoIntelligenceError> {
    let normalized_candidates = candidates
        .iter()
        .cloned()
        .map(build_graph_structural_keyword_overlap_pair_candidate_inputs_from_raw)
        .collect::<Vec<_>>();
    build_graph_structural_keyword_overlap_pair_rerank_request_batch(query, &normalized_candidates)
}

fn normalize_non_blank(
    value: impl AsRef<str>,
    field_name: &str,
) -> Result<String, RepoIntelligenceError> {
    let normalized = value.as_ref().trim().to_string();
    if normalized.is_empty() {
        return Err(graph_structural_projection_error(format!(
            "{field_name} must not be blank"
        )));
    }
    Ok(normalized)
}

fn normalize_pair_endpoint_ids(
    left_id: String,
    right_id: String,
) -> Result<(String, String), RepoIntelligenceError> {
    let left_id = normalize_non_blank(left_id, "pair left id")?;
    let right_id = normalize_non_blank(right_id, "pair right id")?;
    if left_id == right_id {
        return Err(graph_structural_projection_error(
            "pair endpoints must not resolve to the same id",
        ));
    }
    Ok((left_id, right_id))
}

fn normalize_string_list(
    values: Vec<String>,
    field_name: &str,
    allow_empty_list: bool,
) -> Result<Vec<String>, RepoIntelligenceError> {
    let mut normalized = Vec::with_capacity(values.len());
    for (index, value) in values.into_iter().enumerate() {
        let normalized_value = value.trim().to_string();
        if normalized_value.is_empty() {
            return Err(graph_structural_projection_error(format!(
                "{field_name} item {index} must not be blank"
            )));
        }
        normalized.push(normalized_value);
    }
    if !allow_empty_list && normalized.is_empty() {
        return Err(graph_structural_projection_error(format!(
            "{field_name} must contain at least one item"
        )));
    }
    Ok(normalized)
}

fn normalize_non_negative_score(
    value: f64,
    field_name: &str,
) -> Result<f64, RepoIntelligenceError> {
    if !value.is_finite() {
        return Err(graph_structural_projection_error(format!(
            "{field_name} must be finite; found {value}"
        )));
    }
    if value < 0.0 {
        return Err(graph_structural_projection_error(format!(
            "{field_name} must be non-negative; found {value}"
        )));
    }
    Ok(value)
}

fn binary_plane_score(matched: bool) -> f64 {
    if matched { 1.0 } else { 0.0 }
}

fn stable_pair_candidate_id(left_id: &str, right_id: &str) -> String {
    if left_id <= right_id {
        format!("pair:{left_id}:{right_id}")
    } else {
        format!("pair:{right_id}:{left_id}")
    }
}

fn graph_structural_projection_error(detail: impl Into<String>) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "invalid Julia graph-structural projection: {}",
            detail.into()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GraphStructuralCandidateSubgraph, GraphStructuralFilterConstraint,
        GraphStructuralKeywordOverlapCandidateInputs, GraphStructuralKeywordOverlapPairInputs,
        GraphStructuralKeywordOverlapPairRequestInputs,
        GraphStructuralKeywordOverlapPairRerankInputs, GraphStructuralKeywordOverlapQueryInputs,
        GraphStructuralKeywordOverlapRawCandidateInputs, GraphStructuralKeywordTagQueryInputs,
        GraphStructuralNodeMetadataInputs, GraphStructuralPairCandidateInputs,
        GraphStructuralQueryAnchor, GraphStructuralQueryContext, GraphStructuralRerankSignals,
        build_graph_structural_filter_request_row,
        build_graph_structural_keyword_overlap_candidate_inputs,
        build_graph_structural_keyword_overlap_pair_candidate_inputs,
        build_graph_structural_keyword_overlap_pair_candidate_inputs_from_raw,
        build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs,
        build_graph_structural_keyword_overlap_pair_request_input,
        build_graph_structural_keyword_overlap_pair_rerank_request_batch,
        build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_inputs,
        build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_metadata,
        build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_raw_candidates,
        build_graph_structural_keyword_overlap_pair_rerank_request_row,
        build_graph_structural_keyword_overlap_pair_rerank_request_row_from_metadata,
        build_graph_structural_keyword_overlap_query_inputs,
        build_graph_structural_keyword_overlap_raw_candidate_inputs,
        build_graph_structural_keyword_tag_pair_rerank_request_row,
        build_graph_structural_keyword_tag_query_context,
        build_graph_structural_keyword_tag_rerank_signals,
        build_graph_structural_pair_candidate_inputs,
        build_graph_structural_pair_candidate_subgraph,
        build_graph_structural_pair_filter_request_row,
        build_graph_structural_pair_rerank_request_row, build_graph_structural_rerank_request_row,
        graph_structural_pair_candidate_id, graph_structural_shared_tag_anchors,
    };
    use crate::{
        build_graph_structural_filter_request_batch, build_graph_structural_rerank_request_batch,
    };

    #[test]
    fn build_graph_structural_rerank_request_row_projects_semantic_dtos() {
        let query = GraphStructuralQueryContext::new(
            "query-1",
            1,
            3,
            vec![
                GraphStructuralQueryAnchor::new("semantic", "symbol:entry")
                    .expect("semantic anchor"),
                GraphStructuralQueryAnchor::new("tag", "core").expect("tag anchor"),
            ],
            vec!["depends_on".to_string()],
        )
        .expect("query context");
        let candidate = GraphStructuralCandidateSubgraph::new(
            "pair:node-1:node-2",
            vec!["node-1".to_string(), "node-2".to_string()],
            vec!["related".to_string()],
        )
        .expect("candidate");
        let signals =
            GraphStructuralRerankSignals::new(0.7, 0.4, 0.2, 0.3).expect("rerank signals");

        let row = build_graph_structural_rerank_request_row(&query, &candidate, &signals);
        let batch = build_graph_structural_rerank_request_batch(&[row.clone()])
            .expect("rerank batch should validate");

        assert_eq!(row.query_id, "query-1");
        assert_eq!(row.candidate_id, "pair:node-1:node-2");
        assert_eq!(row.anchor_planes, vec!["semantic", "tag"]);
        assert_eq!(row.anchor_values, vec!["symbol:entry", "core"]);
        assert_eq!(row.edge_constraint_kinds, vec!["depends_on"]);
        assert_eq!(row.candidate_node_ids, vec!["node-1", "node-2"]);
        assert_eq!(batch.num_rows(), 1);
    }

    #[test]
    fn build_graph_structural_filter_request_row_allows_empty_edge_lists() {
        let query = GraphStructuralQueryContext::new(
            "query-2",
            0,
            2,
            vec![GraphStructuralQueryAnchor::new("keyword", "solver").expect("keyword anchor")],
            Vec::new(),
        )
        .expect("query context");
        let candidate = GraphStructuralCandidateSubgraph::new(
            "candidate-a",
            vec!["node-a".to_string()],
            Vec::new(),
        )
        .expect("candidate");
        let constraint =
            GraphStructuralFilterConstraint::new("boundary-match", 1).expect("constraint");

        let row = build_graph_structural_filter_request_row(&query, &candidate, &constraint);
        let batch = build_graph_structural_filter_request_batch(&[row.clone()])
            .expect("filter batch should validate");

        assert_eq!(row.edge_constraint_kinds, Vec::<String>::new());
        assert_eq!(row.candidate_edge_kinds, Vec::<String>::new());
        assert_eq!(row.constraint_kind, "boundary-match");
        assert_eq!(row.required_boundary_size, 1);
        assert_eq!(batch.num_rows(), 1);
    }

    #[test]
    fn build_graph_structural_pair_candidate_subgraph_normalizes_stable_id() {
        let candidate = build_graph_structural_pair_candidate_subgraph(
            "node-z",
            "node-a",
            vec!["related".to_string()],
        )
        .expect("pair candidate should normalize");

        assert_eq!(candidate.candidate_id(), "pair:node-a:node-z");
        assert_eq!(
            candidate.node_ids(),
            &["node-z".to_string(), "node-a".to_string()]
        );
        assert_eq!(candidate.edge_kinds(), &["related".to_string()]);
    }

    #[test]
    fn build_graph_structural_pair_rerank_request_row_projects_pair_inputs() {
        let query = GraphStructuralQueryContext::new(
            "query-3",
            0,
            2,
            vec![GraphStructuralQueryAnchor::new("keyword", "alpha").expect("keyword anchor")],
            Vec::new(),
        )
        .expect("query context");
        let signals =
            GraphStructuralRerankSignals::new(0.8, 0.1, 1.0, 0.6).expect("rerank signals");

        let row = build_graph_structural_pair_rerank_request_row(
            &query,
            "doc-b",
            "doc-a",
            vec!["semantic_similar".to_string()],
            &signals,
        )
        .expect("pair row should project");
        let batch = build_graph_structural_rerank_request_batch(&[row.clone()])
            .expect("pair rerank batch should validate");

        assert_eq!(row.candidate_id, "pair:doc-a:doc-b");
        assert_eq!(row.candidate_node_ids, vec!["doc-b", "doc-a"]);
        assert_eq!(row.candidate_edge_kinds, vec!["semantic_similar"]);
        assert_eq!(batch.num_rows(), 1);
    }

    #[test]
    fn build_graph_structural_pair_filter_request_row_rejects_duplicate_endpoints() {
        let query = GraphStructuralQueryContext::new(
            "query-4",
            0,
            1,
            vec![GraphStructuralQueryAnchor::new("tag", "core").expect("tag anchor")],
            Vec::new(),
        )
        .expect("query context");
        let constraint =
            GraphStructuralFilterConstraint::new("boundary-match", 1).expect("constraint");

        let error = build_graph_structural_pair_filter_request_row(
            &query,
            "node-a",
            "node-a",
            Vec::new(),
            &constraint,
        )
        .expect_err("pair filter row should reject duplicate endpoints");
        assert!(
            error
                .to_string()
                .contains("pair endpoints must not resolve to the same id"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn build_graph_structural_keyword_tag_query_context_orders_keyword_before_tag() {
        let query = build_graph_structural_keyword_tag_query_context(
            "query-5",
            0,
            2,
            vec![" alpha ".to_string()],
            vec![" core ".to_string(), " graph ".to_string()],
            vec!["depends_on".to_string()],
        )
        .expect("query context should normalize");

        assert_eq!(query.query_id(), "query-5");
        assert_eq!(query.anchors()[0].plane(), "keyword");
        assert_eq!(query.anchors()[0].value(), "alpha");
        assert_eq!(query.anchors()[1].plane(), "tag");
        assert_eq!(query.anchors()[1].value(), "core");
        assert_eq!(query.anchors()[2].plane(), "tag");
        assert_eq!(query.anchors()[2].value(), "graph");
        assert_eq!(query.edge_constraint_kinds(), &["depends_on".to_string()]);
    }

    #[test]
    fn build_graph_structural_keyword_tag_query_context_rejects_empty_anchor_lists() {
        let error = build_graph_structural_keyword_tag_query_context(
            "query-6",
            0,
            1,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .expect_err("query context should reject empty keyword and tag anchors");
        assert!(
            error
                .to_string()
                .contains("at least one query anchor is required"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn build_graph_structural_keyword_tag_rerank_signals_maps_binary_matches() {
        let signals = build_graph_structural_keyword_tag_rerank_signals(0.6, 0.2, true, false)
            .expect("binary match signals should normalize");

        assert_eq!(signals.semantic_score(), 0.6);
        assert_eq!(signals.dependency_score(), 0.2);
        assert_eq!(signals.keyword_score(), 1.0);
        assert_eq!(signals.tag_score(), 0.0);
    }

    #[test]
    fn build_graph_structural_keyword_tag_pair_rerank_request_row_composes_helper_layers() {
        let row = build_graph_structural_keyword_tag_pair_rerank_request_row(
            GraphStructuralKeywordTagQueryInputs::new(
                "query-7",
                1,
                3,
                vec!["alpha".to_string()],
                vec!["core".to_string()],
                vec!["depends_on".to_string()],
            ),
            GraphStructuralPairCandidateInputs::new(
                "node-b",
                "node-a",
                vec!["semantic_similar".to_string()],
            ),
            0.75,
            0.1,
            true,
            true,
        )
        .expect("combined helper should normalize");
        let batch = build_graph_structural_rerank_request_batch(&[row.clone()])
            .expect("combined helper batch should validate");

        assert_eq!(row.query_id, "query-7");
        assert_eq!(row.candidate_id, "pair:node-a:node-b");
        assert_eq!(row.anchor_planes, vec!["keyword", "tag"]);
        assert_eq!(row.anchor_values, vec!["alpha", "core"]);
        assert_eq!(row.candidate_edge_kinds, vec!["semantic_similar"]);
        assert_eq!(row.keyword_score, 1.0);
        assert_eq!(row.tag_score, 1.0);
        assert_eq!(batch.num_rows(), 1);
    }

    #[test]
    fn graph_structural_shared_tag_anchors_preserve_left_order_and_uniqueness() {
        let shared = graph_structural_shared_tag_anchors(
            vec![
                "core".to_string(),
                "alpha".to_string(),
                "core".to_string(),
                "graph".to_string(),
            ],
            vec!["graph".to_string(), "core".to_string(), "delta".to_string()],
        )
        .expect("shared tag anchors should normalize");

        assert_eq!(shared, vec!["core".to_string(), "graph".to_string()]);
    }

    #[test]
    fn build_graph_structural_keyword_overlap_pair_rerank_request_row_computes_tag_overlap() {
        let row = build_graph_structural_keyword_overlap_pair_rerank_request_row(
            GraphStructuralKeywordTagQueryInputs::new(
                "query-8",
                0,
                2,
                vec!["alpha".to_string()],
                Vec::new(),
                Vec::new(),
            ),
            vec!["alpha".to_string(), "core".to_string()],
            vec!["graph".to_string(), "core".to_string()],
            GraphStructuralPairCandidateInputs::new(
                "node-z",
                "node-a",
                vec!["semantic_similar".to_string()],
            ),
            0.8,
            0.0,
            true,
        )
        .expect("tag-overlap pair helper should normalize");
        let batch = build_graph_structural_rerank_request_batch(&[row.clone()])
            .expect("tag-overlap batch should validate");

        assert_eq!(row.candidate_id, "pair:node-a:node-z");
        assert_eq!(row.anchor_planes, vec!["keyword", "tag"]);
        assert_eq!(row.anchor_values, vec!["alpha", "core"]);
        assert_eq!(row.keyword_score, 1.0);
        assert_eq!(row.tag_score, 1.0);
        assert_eq!(batch.num_rows(), 1);
    }

    #[test]
    fn build_graph_structural_keyword_overlap_pair_rerank_request_row_from_metadata_composes() {
        let row = build_graph_structural_keyword_overlap_pair_rerank_request_row_from_metadata(
            GraphStructuralKeywordOverlapPairInputs::new(
                GraphStructuralKeywordTagQueryInputs::new(
                    "query-9",
                    0,
                    2,
                    vec!["alpha".to_string()],
                    Vec::new(),
                    vec!["semantic_similar".to_string()],
                ),
                GraphStructuralNodeMetadataInputs::new(vec![
                    "alpha".to_string(),
                    "core".to_string(),
                ]),
                GraphStructuralNodeMetadataInputs::new(vec![
                    "graph".to_string(),
                    "core".to_string(),
                ]),
                GraphStructuralPairCandidateInputs::new(
                    "node-k",
                    "node-a",
                    vec!["semantic_similar".to_string()],
                ),
            ),
            0.9,
            0.1,
            true,
        )
        .expect("metadata-aware overlap helper should normalize");
        let batch = build_graph_structural_rerank_request_batch(&[row.clone()])
            .expect("metadata-aware overlap batch should validate");

        assert_eq!(row.query_id, "query-9");
        assert_eq!(row.candidate_id, "pair:node-a:node-k");
        assert_eq!(row.anchor_planes, vec!["keyword", "tag"]);
        assert_eq!(row.anchor_values, vec!["alpha", "core"]);
        assert_eq!(row.edge_constraint_kinds, vec!["semantic_similar"]);
        assert_eq!(row.keyword_score, 1.0);
        assert_eq!(row.tag_score, 1.0);
        assert_eq!(batch.num_rows(), 1);
    }

    #[test]
    fn build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_metadata_composes() {
        let batch =
            build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_metadata(&[
                GraphStructuralKeywordOverlapPairRerankInputs::new(
                    GraphStructuralKeywordOverlapPairInputs::new(
                        GraphStructuralKeywordTagQueryInputs::new(
                            "query-10",
                            1,
                            2,
                            vec!["alpha".to_string()],
                            Vec::new(),
                            Vec::new(),
                        ),
                        GraphStructuralNodeMetadataInputs::new(vec![
                            "alpha".to_string(),
                            "core".to_string(),
                        ]),
                        GraphStructuralNodeMetadataInputs::new(vec![
                            "graph".to_string(),
                            "core".to_string(),
                        ]),
                        GraphStructuralPairCandidateInputs::new("node-r", "node-a", Vec::new()),
                    ),
                    0.5,
                    0.0,
                    true,
                ),
            ])
            .expect("metadata-aware batch helper should normalize");

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(batch.num_columns(), 13);
    }

    #[test]
    fn build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_inputs_composes() {
        let batch =
            build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_inputs(&[
                GraphStructuralKeywordOverlapPairRequestInputs::new(
                    GraphStructuralKeywordTagQueryInputs::new(
                        "query-11",
                        0,
                        1,
                        vec!["alpha".to_string()],
                        Vec::new(),
                        Vec::new(),
                    ),
                    GraphStructuralNodeMetadataInputs::new(vec![
                        "alpha".to_string(),
                        "core".to_string(),
                    ]),
                    GraphStructuralNodeMetadataInputs::new(vec![
                        "core".to_string(),
                        "graph".to_string(),
                    ]),
                    GraphStructuralPairCandidateInputs::new("node-left", "node-right", Vec::new()),
                    0.7,
                    0.0,
                    true,
                ),
            ])
            .expect("higher-level candidate input helper should normalize");

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(batch.num_columns(), 13);
    }

    #[test]
    fn build_graph_structural_keyword_overlap_pair_request_input_composes() {
        let request = build_graph_structural_keyword_overlap_pair_request_input(
            &build_graph_structural_keyword_overlap_query_inputs(
                "query-12",
                1,
                2,
                vec!["alpha".to_string()],
                vec!["semantic_similar".to_string()],
            ),
            GraphStructuralKeywordOverlapCandidateInputs::new(
                GraphStructuralNodeMetadataInputs::new(vec![
                    "alpha".to_string(),
                    "core".to_string(),
                ]),
                GraphStructuralNodeMetadataInputs::new(vec!["core".to_string()]),
                GraphStructuralPairCandidateInputs::new(
                    "node-left",
                    "node-right",
                    vec!["semantic_similar".to_string()],
                ),
                0.6,
                0.2,
                true,
            ),
        );

        assert_eq!(request.metadata_inputs.query_inputs.query_id, "query-12");
        assert_eq!(
            request.metadata_inputs.query_inputs.keyword_anchors,
            vec!["alpha"]
        );
        assert_eq!(request.metadata_inputs.pair_inputs.left_id, "node-left");
        assert_eq!(request.metadata_inputs.pair_inputs.right_id, "node-right");
        assert_eq!(request.semantic_score, 0.6);
    }

    #[test]
    fn build_graph_structural_keyword_overlap_query_inputs_composes() {
        let query = build_graph_structural_keyword_overlap_query_inputs(
            "query-12b",
            1,
            2,
            vec!["alpha".to_string()],
            vec!["semantic_similar".to_string()],
        );

        assert_eq!(
            query,
            GraphStructuralKeywordOverlapQueryInputs::new(
                "query-12b",
                1,
                2,
                vec!["alpha".to_string()],
                vec!["semantic_similar".to_string()],
            )
        );
    }

    #[test]
    fn build_graph_structural_pair_candidate_inputs_composes() {
        let pair_inputs = build_graph_structural_pair_candidate_inputs(
            "node-left",
            "node-right",
            vec!["semantic_similar".to_string()],
        );

        assert_eq!(
            pair_inputs,
            GraphStructuralPairCandidateInputs::new(
                "node-left",
                "node-right",
                vec!["semantic_similar".to_string()],
            )
        );
    }

    #[test]
    fn build_graph_structural_keyword_overlap_pair_candidate_inputs_composes() {
        let candidate = build_graph_structural_keyword_overlap_pair_candidate_inputs(
            build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
                "node-left",
                "node-right",
                vec!["semantic_similar".to_string()],
                vec!["alpha".to_string(), "core".to_string()],
                vec!["core".to_string()],
            ),
            0.6,
            0.2,
            true,
        );

        assert_eq!(
            candidate,
            GraphStructuralKeywordOverlapCandidateInputs::new(
                GraphStructuralNodeMetadataInputs::new(vec![
                    "alpha".to_string(),
                    "core".to_string(),
                ]),
                GraphStructuralNodeMetadataInputs::new(vec!["core".to_string()]),
                GraphStructuralPairCandidateInputs::new(
                    "node-left",
                    "node-right",
                    vec!["semantic_similar".to_string()],
                ),
                0.6,
                0.2,
                true,
            )
        );
    }

    #[test]
    fn build_graph_structural_keyword_overlap_pair_candidate_inputs_from_raw_composes() {
        let candidate = build_graph_structural_keyword_overlap_pair_candidate_inputs_from_raw(
            "node-left",
            "node-right",
            vec!["semantic_similar".to_string()],
            vec!["alpha".to_string(), "core".to_string()],
            vec!["core".to_string()],
            0.6,
            0.2,
            true,
        );

        assert_eq!(
            candidate,
            GraphStructuralKeywordOverlapCandidateInputs::new(
                GraphStructuralNodeMetadataInputs::new(vec![
                    "alpha".to_string(),
                    "core".to_string(),
                ]),
                GraphStructuralNodeMetadataInputs::new(vec!["core".to_string()]),
                GraphStructuralPairCandidateInputs::new(
                    "node-left",
                    "node-right",
                    vec!["semantic_similar".to_string()],
                ),
                0.6,
                0.2,
                true,
            )
        );
    }

    #[test]
    fn build_graph_structural_keyword_overlap_raw_candidate_inputs_composes() {
        let candidate = build_graph_structural_keyword_overlap_raw_candidate_inputs(
            "node-left",
            "node-right",
            vec!["semantic_similar".to_string()],
            vec!["alpha".to_string(), "core".to_string()],
            vec!["core".to_string()],
            0.6,
            0.2,
            true,
        );

        assert_eq!(
            candidate,
            GraphStructuralKeywordOverlapRawCandidateInputs::new(
                "node-left",
                "node-right",
                vec!["semantic_similar".to_string()],
                vec!["alpha".to_string(), "core".to_string()],
                vec!["core".to_string()],
                0.6,
                0.2,
                true,
            )
        );
    }

    #[test]
    fn graph_structural_keyword_overlap_candidate_inputs_new_composes() {
        let candidate = build_graph_structural_keyword_overlap_candidate_inputs(
            build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
                "node-left",
                "node-right",
                vec!["semantic_similar".to_string()],
                vec!["alpha".to_string(), "core".to_string()],
                vec!["core".to_string()],
            ),
            0.6,
            0.2,
            true,
        );

        assert_eq!(
            candidate,
            GraphStructuralKeywordOverlapCandidateInputs::new(
                GraphStructuralNodeMetadataInputs::new(vec![
                    "alpha".to_string(),
                    "core".to_string(),
                ]),
                GraphStructuralNodeMetadataInputs::new(vec!["core".to_string()]),
                GraphStructuralPairCandidateInputs::new(
                    "node-left",
                    "node-right",
                    vec!["semantic_similar".to_string()],
                ),
                0.6,
                0.2,
                true,
            )
        );
    }

    #[test]
    fn build_graph_structural_keyword_overlap_candidate_inputs_composes() {
        let candidate = build_graph_structural_keyword_overlap_candidate_inputs(
            build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
                "node-left",
                "node-right",
                vec!["semantic_similar".to_string()],
                vec!["alpha".to_string(), "core".to_string()],
                vec!["core".to_string()],
            ),
            0.6,
            0.2,
            true,
        );

        assert_eq!(
            candidate,
            GraphStructuralKeywordOverlapCandidateInputs::new(
                GraphStructuralNodeMetadataInputs::new(vec![
                    "alpha".to_string(),
                    "core".to_string(),
                ]),
                GraphStructuralNodeMetadataInputs::new(vec!["core".to_string()]),
                GraphStructuralPairCandidateInputs::new(
                    "node-left",
                    "node-right",
                    vec!["semantic_similar".to_string()],
                ),
                0.6,
                0.2,
                true,
            )
        );
    }

    #[test]
    fn build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_raw_candidates_composes()
     {
        let batch =
            build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_raw_candidates(
                &build_graph_structural_keyword_overlap_query_inputs(
                    "query-13a",
                    0,
                    1,
                    vec!["alpha".to_string()],
                    Vec::new(),
                ),
                &[build_graph_structural_keyword_overlap_raw_candidate_inputs(
                    "node-left",
                    "node-right",
                    Vec::new(),
                    vec!["alpha".to_string(), "core".to_string()],
                    vec!["core".to_string(), "graph".to_string()],
                    0.7,
                    0.0,
                    true,
                )],
            )
            .expect("raw candidate batch helper should normalize");

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(batch.num_columns(), 13);
    }

    #[test]
    fn build_graph_structural_keyword_overlap_pair_rerank_request_batch_composes() {
        let batch = build_graph_structural_keyword_overlap_pair_rerank_request_batch(
            &build_graph_structural_keyword_overlap_query_inputs(
                "query-13",
                0,
                1,
                vec!["alpha".to_string()],
                Vec::new(),
            ),
            &[
                build_graph_structural_keyword_overlap_pair_candidate_inputs(
                    build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
                        "node-left",
                        "node-right",
                        Vec::new(),
                        vec!["alpha".to_string(), "core".to_string()],
                        vec!["core".to_string(), "graph".to_string()],
                    ),
                    0.7,
                    0.0,
                    true,
                ),
            ],
        )
        .expect("query-candidate batch helper should normalize");

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(batch.num_columns(), 13);
    }

    #[test]
    fn graph_structural_query_context_rejects_empty_anchor_list() {
        let error = GraphStructuralQueryContext::new("query-1", 0, 1, Vec::new(), Vec::new())
            .expect_err("query context should reject empty anchors");
        assert!(
            error
                .to_string()
                .contains("at least one query anchor is required"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn graph_structural_candidate_subgraph_rejects_blank_node_ids() {
        let error = GraphStructuralCandidateSubgraph::new(
            "candidate-a",
            vec!["node-a".to_string(), "  ".to_string()],
            Vec::new(),
        )
        .expect_err("candidate should reject blank node ids");
        assert!(
            error
                .to_string()
                .contains("candidate node ids item 1 must not be blank"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn graph_structural_rerank_signals_reject_negative_scores() {
        let error = GraphStructuralRerankSignals::new(-0.1, 0.0, 0.0, 0.0)
            .expect_err("signals should reject negative scores");
        assert!(
            error
                .to_string()
                .contains("semantic score must be non-negative"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn graph_structural_pair_candidate_id_rejects_duplicate_endpoints() {
        let error = graph_structural_pair_candidate_id("same-node", "same-node")
            .expect_err("pair candidate id should reject duplicate endpoints");
        assert!(
            error
                .to_string()
                .contains("pair endpoints must not resolve to the same id"),
            "unexpected error: {error}"
        );
    }
}
