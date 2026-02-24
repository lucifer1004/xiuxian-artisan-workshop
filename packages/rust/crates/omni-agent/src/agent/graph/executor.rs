use anyhow::{Result, anyhow, bail};
use std::collections::BTreeSet;
use std::time::Instant;

use crate::contracts::{
    GraphExecutionPlan, GraphPlanStep, GraphPlanStepKind, OmegaDecision, RouteTrace,
    RouteTraceGraphStep, RouteTraceInjection,
};
use crate::shortcuts::WorkflowBridgeMode;

use super::super::graph_bridge;
use super::super::omega::ShortcutFallbackAction;
use super::super::{Agent, ToolExecutionSummary};

/// Input bundle for deterministic graph-plan execution.
#[derive(Debug, Clone)]
pub(in crate::agent) struct GraphPlanExecutionInput {
    /// Source shortcut mode (`graph` or `omega`) for telemetry/fallback records.
    pub workflow_mode: WorkflowBridgeMode,
    /// Runtime turn identifier shared with route trace and reflection.
    pub turn_id: u64,
    /// Original shortcut message; used when fallback reroutes into `ReAct`.
    pub shortcut_user_message: String,
    /// First-attempt bridge args (usually enriched with `_omni` metadata).
    pub bridge_arguments_with_metadata: Option<serde_json::Value>,
    /// Metadata-free retry args used by compatibility fallback.
    pub bridge_arguments_without_metadata: Option<serde_json::Value>,
    /// Optional qianhuan injection summary for route trace payload.
    pub injection: Option<RouteTraceInjection>,
}

/// Deterministic graph-plan execution result.
#[derive(Debug, Clone)]
pub(in crate::agent) enum GraphPlanExecutionOutcome {
    Completed {
        output: String,
        tool_summary: ToolExecutionSummary,
    },
    RouteToReact {
        rewritten_user_message: String,
        tool_summary: ToolExecutionSummary,
    },
}

/// Graph-plan execution error carrying tool-attempt summary for memory feedback.
#[derive(Debug)]
pub(in crate::agent) struct GraphPlanExecutionError {
    pub(in crate::agent) error: anyhow::Error,
    pub(in crate::agent) tool_summary: ToolExecutionSummary,
}

impl Agent {
    #[allow(clippy::too_many_lines)]
    pub(in crate::agent) async fn execute_graph_shortcut_plan(
        &self,
        session_id: &str,
        decision: &OmegaDecision,
        plan: &GraphExecutionPlan,
        input: GraphPlanExecutionInput,
    ) -> std::result::Result<GraphPlanExecutionOutcome, GraphPlanExecutionError> {
        let execution_started = Instant::now();
        let mut tool_summary = ToolExecutionSummary::default();
        let mut invoke_output: Option<String> = None;
        let mut invoke_error: Option<anyhow::Error> = None;
        let mut invoke_seen = false;
        let mut invoked_tool_name = plan.tool_name.clone();
        let mut step_traces = Vec::<RouteTraceGraphStep>::new();
        let mut failure_taxonomy = BTreeSet::<String>::new();
        let mut fallback_applied = false;

        let ordered_steps = match ordered_steps(plan) {
            Ok(steps) => steps,
            Err(error) => {
                failure_taxonomy.insert(classify_failure_taxonomy(error.to_string().as_str()));
                self.emit_graph_route_trace(
                    session_id,
                    decision,
                    plan,
                    &input,
                    execution_started,
                    fallback_applied,
                    &failure_taxonomy,
                    &step_traces,
                )
                .await;
                return Err(GraphPlanExecutionError {
                    error,
                    tool_summary,
                });
            }
        };

        for step in ordered_steps {
            let step_attempt = match step.kind {
                GraphPlanStepKind::InvokeGraphTool => tool_summary.attempted.saturating_add(1),
                _ => tool_summary.attempted,
            };
            let step_started_at = Instant::now();
            self.record_graph_plan_step_started(session_id, plan, step, step_attempt);

            match step.kind {
                GraphPlanStepKind::PrepareInjectionContext => {
                    self.record_graph_plan_step_succeeded(
                        session_id,
                        plan,
                        step,
                        step_attempt,
                        "prepared",
                    );
                    push_step_trace(
                        &mut step_traces,
                        step,
                        step_attempt,
                        step_started_at,
                        "prepared",
                        None,
                    );
                }
                GraphPlanStepKind::InvokeGraphTool => {
                    if invoke_seen {
                        let error = anyhow!(
                            "graph plan `{}` contains duplicate invoke_graph_tool step",
                            plan.plan_id
                        );
                        self.record_graph_plan_step_failed(
                            session_id,
                            plan,
                            step,
                            step_attempt,
                            &error,
                        );
                        failure_taxonomy
                            .insert(classify_failure_taxonomy(error.to_string().as_str()));
                        push_step_trace(
                            &mut step_traces,
                            step,
                            step_attempt,
                            step_started_at,
                            "failed_duplicate_invoke_step",
                            Some(error.to_string()),
                        );
                        self.emit_graph_route_trace(
                            session_id,
                            decision,
                            plan,
                            &input,
                            execution_started,
                            fallback_applied,
                            &failure_taxonomy,
                            &step_traces,
                        )
                        .await;
                        return Err(GraphPlanExecutionError {
                            error,
                            tool_summary,
                        });
                    }
                    invoke_seen = true;

                    let step_tool_name = step
                        .tool_name
                        .as_deref()
                        .unwrap_or(plan.tool_name.as_str())
                        .trim();
                    if step_tool_name.is_empty() {
                        let error = anyhow!(
                            "graph plan `{}` invoke step has empty tool name",
                            plan.plan_id
                        );
                        self.record_graph_plan_step_failed(
                            session_id,
                            plan,
                            step,
                            step_attempt,
                            &error,
                        );
                        failure_taxonomy
                            .insert(classify_failure_taxonomy(error.to_string().as_str()));
                        push_step_trace(
                            &mut step_traces,
                            step,
                            step_attempt,
                            step_started_at,
                            "failed_empty_tool_name",
                            Some(error.to_string()),
                        );
                        self.emit_graph_route_trace(
                            session_id,
                            decision,
                            plan,
                            &input,
                            execution_started,
                            fallback_applied,
                            &failure_taxonomy,
                            &step_traces,
                        )
                        .await;
                        return Err(GraphPlanExecutionError {
                            error,
                            tool_summary,
                        });
                    }
                    invoked_tool_name = step_tool_name.to_string();

                    let initial_request = graph_bridge::GraphBridgeRequest {
                        tool_name: invoked_tool_name.clone(),
                        arguments: input.bridge_arguments_with_metadata.clone(),
                    };
                    match self.execute_graph_bridge(initial_request).await {
                        Ok(result) => {
                            tool_summary.record_result(result.is_error);
                            invoke_output = Some(result.output);
                            if result.is_error {
                                failure_taxonomy.insert("tool_error_payload".to_string());
                            }
                            let status = if result.is_error {
                                "tool_returned_error_payload"
                            } else {
                                "tool_call_succeeded"
                            };
                            self.record_graph_plan_step_succeeded(
                                session_id,
                                plan,
                                step,
                                tool_summary.attempted,
                                status,
                            );
                            push_step_trace(
                                &mut step_traces,
                                step,
                                tool_summary.attempted,
                                step_started_at,
                                status,
                                None,
                            );
                        }
                        Err(error) => {
                            tool_summary.record_transport_failure();
                            self.record_graph_plan_step_failed(
                                session_id,
                                plan,
                                step,
                                tool_summary.attempted,
                                &error,
                            );
                            failure_taxonomy
                                .insert(classify_failure_taxonomy(error.to_string().as_str()));
                            push_step_trace(
                                &mut step_traces,
                                step,
                                tool_summary.attempted,
                                step_started_at,
                                "tool_call_transport_failed",
                                Some(error.to_string()),
                            );
                            invoke_error = Some(error);
                        }
                    }
                }
                GraphPlanStepKind::EvaluateFallback => {
                    if invoke_output.is_some() {
                        self.record_graph_plan_step_succeeded(
                            session_id,
                            plan,
                            step,
                            step_attempt,
                            "skipped_no_bridge_error",
                        );
                        push_step_trace(
                            &mut step_traces,
                            step,
                            step_attempt,
                            step_started_at,
                            "skipped_no_bridge_error",
                            None,
                        );
                        continue;
                    }

                    let Some(initial_error) = invoke_error.take() else {
                        self.record_graph_plan_step_succeeded(
                            session_id,
                            plan,
                            step,
                            step_attempt,
                            "skipped_no_transport_error",
                        );
                        push_step_trace(
                            &mut step_traces,
                            step,
                            step_attempt,
                            step_started_at,
                            "skipped_no_transport_error",
                            None,
                        );
                        continue;
                    };

                    let fallback_action = match fallback_action_from_step(step) {
                        Ok(action) => action,
                        Err(error) => {
                            self.record_graph_plan_step_failed(
                                session_id,
                                plan,
                                step,
                                step_attempt,
                                &error,
                            );
                            failure_taxonomy
                                .insert(classify_failure_taxonomy(error.to_string().as_str()));
                            push_step_trace(
                                &mut step_traces,
                                step,
                                step_attempt,
                                step_started_at,
                                "failed_invalid_fallback_action",
                                Some(error.to_string()),
                            );
                            self.emit_graph_route_trace(
                                session_id,
                                decision,
                                plan,
                                &input,
                                execution_started,
                                fallback_applied,
                                &failure_taxonomy,
                                &step_traces,
                            )
                            .await;
                            return Err(GraphPlanExecutionError {
                                error,
                                tool_summary,
                            });
                        }
                    };

                    match fallback_action {
                        ShortcutFallbackAction::RetryBridgeWithoutMetadata => {
                            fallback_applied = true;
                            self.record_shortcut_fallback(
                                session_id,
                                decision,
                                input.workflow_mode,
                                invoked_tool_name.as_str(),
                                ShortcutFallbackAction::RetryBridgeWithoutMetadata,
                                &initial_error,
                            );
                            match self
                                .execute_graph_bridge(graph_bridge::GraphBridgeRequest {
                                    tool_name: invoked_tool_name.clone(),
                                    arguments: input.bridge_arguments_without_metadata.clone(),
                                })
                                .await
                            {
                                Ok(result) => {
                                    tool_summary.record_result(result.is_error);
                                    invoke_output = Some(result.output);
                                    if result.is_error {
                                        failure_taxonomy.insert("tool_error_payload".to_string());
                                    }
                                    let status = if result.is_error {
                                        "retry_returned_error_payload"
                                    } else {
                                        "retry_succeeded_without_metadata"
                                    };
                                    self.record_graph_plan_step_succeeded(
                                        session_id,
                                        plan,
                                        step,
                                        tool_summary.attempted,
                                        status,
                                    );
                                    push_step_trace(
                                        &mut step_traces,
                                        step,
                                        tool_summary.attempted,
                                        step_started_at,
                                        status,
                                        None,
                                    );
                                }
                                Err(retry_error) => {
                                    tool_summary.record_transport_failure();
                                    self.record_graph_plan_step_failed(
                                        session_id,
                                        plan,
                                        step,
                                        tool_summary.attempted,
                                        &retry_error,
                                    );
                                    failure_taxonomy.insert(classify_failure_taxonomy(
                                        retry_error.to_string().as_str(),
                                    ));
                                    push_step_trace(
                                        &mut step_traces,
                                        step,
                                        tool_summary.attempted,
                                        step_started_at,
                                        "retry_transport_failed",
                                        Some(retry_error.to_string()),
                                    );
                                    self.emit_graph_route_trace(
                                        session_id,
                                        decision,
                                        plan,
                                        &input,
                                        execution_started,
                                        fallback_applied,
                                        &failure_taxonomy,
                                        &step_traces,
                                    )
                                    .await;
                                    return Err(GraphPlanExecutionError {
                                        error: retry_error,
                                        tool_summary,
                                    });
                                }
                            }
                        }
                        ShortcutFallbackAction::RouteToReact => {
                            fallback_applied = true;
                            self.record_shortcut_fallback(
                                session_id,
                                decision,
                                input.workflow_mode,
                                invoked_tool_name.as_str(),
                                ShortcutFallbackAction::RouteToReact,
                                &initial_error,
                            );
                            self.record_graph_plan_step_succeeded(
                                session_id,
                                plan,
                                step,
                                tool_summary.attempted,
                                "rerouted_to_react",
                            );
                            failure_taxonomy.insert(classify_failure_taxonomy(
                                initial_error.to_string().as_str(),
                            ));
                            push_step_trace(
                                &mut step_traces,
                                step,
                                tool_summary.attempted,
                                step_started_at,
                                "rerouted_to_react",
                                Some(initial_error.to_string()),
                            );
                            self.record_graph_execution_rerouted(
                                session_id,
                                plan,
                                "react",
                                "bridge_transport_error",
                            );
                            self.emit_graph_route_trace(
                                session_id,
                                decision,
                                plan,
                                &input,
                                execution_started,
                                fallback_applied,
                                &failure_taxonomy,
                                &step_traces,
                            )
                            .await;
                            return Ok(GraphPlanExecutionOutcome::RouteToReact {
                                rewritten_user_message: format!(
                                    "Execute this task with ReAct because workflow bridge failed: {}",
                                    input.shortcut_user_message
                                ),
                                tool_summary,
                            });
                        }
                        ShortcutFallbackAction::Abort => {
                            fallback_applied = true;
                            self.record_shortcut_fallback(
                                session_id,
                                decision,
                                input.workflow_mode,
                                invoked_tool_name.as_str(),
                                ShortcutFallbackAction::Abort,
                                &initial_error,
                            );
                            self.record_graph_plan_step_failed(
                                session_id,
                                plan,
                                step,
                                tool_summary.attempted,
                                &initial_error,
                            );
                            failure_taxonomy.insert(classify_failure_taxonomy(
                                initial_error.to_string().as_str(),
                            ));
                            push_step_trace(
                                &mut step_traces,
                                step,
                                tool_summary.attempted,
                                step_started_at,
                                "fallback_abort",
                                Some(initial_error.to_string()),
                            );
                            self.emit_graph_route_trace(
                                session_id,
                                decision,
                                plan,
                                &input,
                                execution_started,
                                fallback_applied,
                                &failure_taxonomy,
                                &step_traces,
                            )
                            .await;
                            return Err(GraphPlanExecutionError {
                                error: initial_error,
                                tool_summary,
                            });
                        }
                    }
                }
            }
        }

        if let Some(output) = invoke_output {
            self.record_graph_execution_completed(
                session_id,
                plan,
                tool_summary.attempted,
                output.len(),
            );
            self.emit_graph_route_trace(
                session_id,
                decision,
                plan,
                &input,
                execution_started,
                fallback_applied,
                &failure_taxonomy,
                &step_traces,
            )
            .await;
            return Ok(GraphPlanExecutionOutcome::Completed {
                output,
                tool_summary,
            });
        }

        if let Some(error) = invoke_error {
            failure_taxonomy.insert(classify_failure_taxonomy(error.to_string().as_str()));
            self.emit_graph_route_trace(
                session_id,
                decision,
                plan,
                &input,
                execution_started,
                fallback_applied,
                &failure_taxonomy,
                &step_traces,
            )
            .await;
            return Err(GraphPlanExecutionError {
                error,
                tool_summary,
            });
        }

        if !invoke_seen {
            let error = anyhow!(
                "graph plan `{}` did not include invoke_graph_tool step",
                plan.plan_id
            );
            failure_taxonomy.insert(classify_failure_taxonomy(error.to_string().as_str()));
            self.emit_graph_route_trace(
                session_id,
                decision,
                plan,
                &input,
                execution_started,
                fallback_applied,
                &failure_taxonomy,
                &step_traces,
            )
            .await;
            return Err(GraphPlanExecutionError {
                error,
                tool_summary,
            });
        }

        let error = anyhow!(
            "graph plan `{}` finished without bridge output or fallback",
            plan.plan_id
        );
        failure_taxonomy.insert(classify_failure_taxonomy(error.to_string().as_str()));
        self.emit_graph_route_trace(
            session_id,
            decision,
            plan,
            &input,
            execution_started,
            fallback_applied,
            &failure_taxonomy,
            &step_traces,
        )
        .await;
        Err(GraphPlanExecutionError {
            error,
            tool_summary,
        })
    }

    #[allow(clippy::too_many_arguments)]
    async fn emit_graph_route_trace(
        &self,
        session_id: &str,
        decision: &OmegaDecision,
        plan: &GraphExecutionPlan,
        input: &GraphPlanExecutionInput,
        execution_started: Instant,
        fallback_applied: bool,
        failure_taxonomy: &BTreeSet<String>,
        step_traces: &[RouteTraceGraphStep],
    ) {
        let trace = RouteTrace {
            session_id: session_id.to_string(),
            turn_id: input.turn_id,
            selected_route: decision.route,
            confidence: decision.confidence,
            risk_level: decision.risk_level,
            tool_trust_class: decision.tool_trust_class,
            fallback_applied: Some(fallback_applied),
            fallback_policy: Some(decision.fallback_policy),
            tool_chain: derive_tool_chain(plan),
            latency_ms: Some(execution_started.elapsed().as_secs_f64() * 1000.0),
            failure_taxonomy: failure_taxonomy.iter().cloned().collect(),
            injection: input.injection.clone(),
            plan_id: Some(plan.plan_id.clone()),
            workflow_mode: Some(plan.workflow_mode),
            graph_steps: (!step_traces.is_empty()).then(|| step_traces.to_vec()),
        };
        self.record_route_trace(&trace).await;
    }
}

fn push_step_trace(
    traces: &mut Vec<RouteTraceGraphStep>,
    step: &GraphPlanStep,
    attempt: u32,
    started_at: Instant,
    status: &str,
    failure_reason: Option<String>,
) {
    traces.push(RouteTraceGraphStep {
        index: step.index,
        id: step.id.clone(),
        kind: step.kind,
        attempt,
        latency_ms: started_at.elapsed().as_secs_f64() * 1000.0,
        status: status.to_string(),
        failure_reason,
        tool_name: step.tool_name.clone(),
        fallback_action: step.fallback_action.clone(),
    });
}

fn derive_tool_chain(plan: &GraphExecutionPlan) -> Vec<String> {
    let mut chain = Vec::<String>::new();
    for tool in plan
        .steps
        .iter()
        .filter_map(|step| step.tool_name.as_deref())
    {
        if !tool.trim().is_empty() && !chain.iter().any(|existing| existing == tool) {
            chain.push(tool.to_string());
        }
    }
    if chain.is_empty() {
        chain.push(plan.tool_name.clone());
    }
    chain
}

fn classify_failure_taxonomy(reason: &str) -> String {
    let lower = reason.to_ascii_lowercase();
    if lower.contains("timeout") || lower.contains("timed out") {
        return "timeout".to_string();
    }
    if lower.contains("connection")
        || lower.contains("connect")
        || lower.contains("transport")
        || lower.contains("send")
        || lower.contains("broken pipe")
        || lower.contains("refused")
    {
        return "transport".to_string();
    }
    if lower.contains("schema")
        || lower.contains("invalid")
        || lower.contains("must")
        || lower.contains("unsupported")
        || lower.contains("graph plan")
    {
        return "validation".to_string();
    }
    if lower.contains("tool") && lower.contains("error") {
        return "tool_error_payload".to_string();
    }
    "execution_error".to_string()
}

fn ordered_steps(plan: &GraphExecutionPlan) -> Result<Vec<&GraphPlanStep>> {
    if let Err(error) = plan.validate_shortcut_contract() {
        bail!("{error}");
    }

    let mut ordered: Vec<&GraphPlanStep> = plan.steps.iter().collect();
    ordered.sort_by_key(|step| step.index);

    Ok(ordered)
}

fn fallback_action_from_step(step: &GraphPlanStep) -> Result<ShortcutFallbackAction> {
    match step.fallback_action.as_deref() {
        Some("abort") => Ok(ShortcutFallbackAction::Abort),
        Some("retry_bridge_without_metadata") => {
            Ok(ShortcutFallbackAction::RetryBridgeWithoutMetadata)
        }
        Some("route_to_react") => Ok(ShortcutFallbackAction::RouteToReact),
        Some(other) => bail!(
            "graph plan step `{}` contains unsupported fallback action `{}`",
            step.id,
            other
        ),
        None => bail!(
            "graph plan step `{}` missing fallback_action in evaluate_fallback",
            step.id
        ),
    }
}

#[cfg(test)]
#[path = "../../../tests/agent/graph_executor.rs"]
mod tests;
