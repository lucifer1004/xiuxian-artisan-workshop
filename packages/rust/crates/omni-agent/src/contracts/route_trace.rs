use serde::{Deserialize, Serialize};

use super::{
    GraphPlanStepKind, GraphWorkflowMode, OmegaFallbackPolicy, OmegaRiskLevel, OmegaRoute,
    OmegaToolTrustClass,
};

/// Aggregated per-step route trace emitted once per turn execution outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteTrace {
    pub session_id: String,
    pub turn_id: u64,
    pub selected_route: OmegaRoute,
    pub confidence: f32,
    pub risk_level: OmegaRiskLevel,
    pub tool_trust_class: OmegaToolTrustClass,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_applied: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_policy: Option<OmegaFallbackPolicy>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tool_chain: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub failure_taxonomy: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub injection: Option<RouteTraceInjection>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_mode: Option<GraphWorkflowMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub graph_steps: Option<Vec<RouteTraceGraphStep>>,
}

/// Injection summary attached to route trace when qianhuan context exists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteTraceInjection {
    pub blocks_used: u64,
    pub chars_injected: u64,
    pub dropped_by_budget: u64,
}

/// Per-step execution trace for deterministic graph routes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteTraceGraphStep {
    pub index: u8,
    pub id: String,
    pub kind: GraphPlanStepKind,
    pub attempt: u32,
    pub latency_ms: f64,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failure_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_action: Option<String>,
}
