use crate::contracts::{
    GraphExecutionPlan, GraphPlanStep, GraphPlanStepKind, GraphWorkflowMode, OmegaDecision,
};
use crate::shortcuts::WorkflowBridgeMode;

use super::super::omega;

/// Build a deterministic graph execution plan for workflow bridge shortcuts.
#[must_use]
pub(crate) fn build_shortcut_plan(
    workflow_mode: WorkflowBridgeMode,
    decision: &OmegaDecision,
    tool_name: &str,
) -> GraphExecutionPlan {
    let workflow_mode = match workflow_mode {
        WorkflowBridgeMode::Graph => GraphWorkflowMode::Graph,
        WorkflowBridgeMode::Omega => GraphWorkflowMode::Omega,
    };
    let fallback_action = omega::resolve_shortcut_fallback(decision, 0)
        .as_str()
        .to_string();
    let normalized_tool = tool_name.trim();

    let steps = vec![
        GraphPlanStep {
            index: 1,
            id: "prepare_injection_context".to_string(),
            kind: GraphPlanStepKind::PrepareInjectionContext,
            description: "Build and attach typed injection snapshot before graph bridge execution."
                .to_string(),
            tool_name: None,
            fallback_action: None,
        },
        GraphPlanStep {
            index: 2,
            id: "invoke_graph_tool".to_string(),
            kind: GraphPlanStepKind::InvokeGraphTool,
            description: "Invoke configured MCP graph bridge tool.".to_string(),
            tool_name: Some(normalized_tool.to_string()),
            fallback_action: None,
        },
        GraphPlanStep {
            index: 3,
            id: "evaluate_fallback".to_string(),
            kind: GraphPlanStepKind::EvaluateFallback,
            description: "Apply deterministic fallback action when bridge execution fails."
                .to_string(),
            tool_name: None,
            fallback_action: Some(fallback_action.clone()),
        },
    ];

    let plan_id = format!(
        "graph-plan:{}:{}:{}:{}",
        workflow_mode.as_str(),
        normalized_tool,
        decision.fallback_policy.as_str(),
        decision.tool_trust_class.as_str()
    );

    let plan = GraphExecutionPlan {
        plan_id,
        plan_version: "v1".to_string(),
        route: decision.route,
        workflow_mode,
        tool_name: normalized_tool.to_string(),
        fallback_policy: decision.fallback_policy,
        steps,
    };

    debug_assert!(
        plan.validate_shortcut_contract().is_ok(),
        "planner generated invalid deterministic graph plan contract"
    );

    plan
}

#[cfg(test)]
#[path = "../../../tests/agent/graph_planner.rs"]
mod tests;
