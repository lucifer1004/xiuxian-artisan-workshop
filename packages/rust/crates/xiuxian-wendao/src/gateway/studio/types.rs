use serde::{Deserialize, Serialize};
use specta::{Type, TypeCollection};

// === VFS Types ===

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VfsEntry {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum VfsCategory {
    Folder,
    Skill,
    Doc,
    Knowledge,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VfsScanEntry {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub category: VfsCategory,
    pub size: u64,
    pub modified: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    pub has_frontmatter: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wendao_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VfsScanResult {
    pub entries: Vec<VfsScanEntry>,
    pub file_count: usize,
    pub dir_count: usize,
    pub scan_duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VfsContentResponse {
    pub path: String,
    pub content: String,
    pub content_type: String,
}

// === Graph Types ===

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct NodeNeighbors {
    pub node_id: String,
    pub name: String,
    pub node_type: String,
    pub incoming: Vec<String>,
    pub outgoing: Vec<String>,
    pub two_hop: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub path: String,
    pub node_type: String,
    pub is_center: bool,
    pub distance: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphLink {
    pub source: String,
    pub target: String,
    pub direction: String,
    pub distance: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphNeighborsResponse {
    pub center: GraphNode,
    pub nodes: Vec<GraphNode>,
    pub links: Vec<GraphLink>,
    pub total_nodes: usize,
    pub total_links: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TopologyNode {
    pub id: String,
    pub name: String,
    pub node_type: String,
    pub position: [f32; 3],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cluster_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TopologyLink {
    pub from: String,
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClusterInfo {
    pub id: String,
    pub name: String,
    pub centroid: [f32; 3],
    pub node_count: usize,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Topology3D {
    pub nodes: Vec<TopologyNode>,
    pub links: Vec<TopologyLink>,
    pub clusters: Vec<ClusterInfo>,
}

// === State Types ===

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum NodeState {
    Idle,
    Active,
    Processing,
    Success,
    Wait,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ResearchStateEvent {
    NodeActivated {
        node_id: String,
        state: NodeState,
    },
    StepStarted {
        step_id: String,
        timestamp: u64,
    },
    StepCompleted {
        step_id: String,
        success: bool,
        duration_ms: u64,
    },
    TopologyUpdated {
        node_count: usize,
        link_count: usize,
    },
}

// === Search Types ===

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSearchResult {
    pub id: String,
    pub name: String,
    pub score: f64,
    pub snippet: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub stem: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_type: Option<String>,
    pub tags: Vec<String>,
    pub score: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_section: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub query: String,
    pub hits: Vec<SearchHit>,
    pub hit_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph_confidence_score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_mode: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum AutocompleteSuggestionType {
    Title,
    Tag,
    Stem,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AutocompleteSuggestion {
    pub text: String,
    pub suggestion_type: AutocompleteSuggestionType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AutocompleteResponse {
    pub prefix: String,
    pub suggestions: Vec<AutocompleteSuggestion>,
}

// === UI Config Types ===

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UiConfig {
    pub index_paths: Vec<String>,
}

// === Error Types ===

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

pub fn studio_type_collection() -> TypeCollection {
    TypeCollection::default()
        .register::<VfsEntry>()
        .register::<VfsCategory>()
        .register::<VfsScanEntry>()
        .register::<VfsScanResult>()
        .register::<VfsContentResponse>()
        .register::<NodeNeighbors>()
        .register::<GraphNode>()
        .register::<GraphLink>()
        .register::<GraphNeighborsResponse>()
        .register::<TopologyNode>()
        .register::<TopologyLink>()
        .register::<ClusterInfo>()
        .register::<Topology3D>()
        .register::<NodeState>()
        .register::<ResearchStateEvent>()
        .register::<KnowledgeSearchResult>()
        .register::<SearchHit>()
        .register::<SearchResponse>()
        .register::<AutocompleteSuggestionType>()
        .register::<AutocompleteSuggestion>()
        .register::<AutocompleteResponse>()
        .register::<UiConfig>()
        .register::<ApiError>()
}
