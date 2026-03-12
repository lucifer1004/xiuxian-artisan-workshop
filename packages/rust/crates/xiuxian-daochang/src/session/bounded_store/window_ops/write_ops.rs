use super::*;

impl Agent {
    fn route_trace_stream_name(&self) -> &str {
        "route.events"
    }

    pub(crate) fn next_runtime_turn_id(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or_default()
    }

    pub(crate) fn record_omega_decision(
        &self,
        session_id: &str,
        decision: &OmegaDecision,
        workflow_mode: Option<WorkflowBridgeMode>,
        tool_name: Option<&str>,
    ) {
        tracing::debug!(
            event = SessionEvent::RouteDecisionSelected.as_str(),
            session_id,
            workflow_mode = workflow_mode.map(WorkflowBridgeMode::as_str),
            tool_name,
            route = decision.route.as_str(),
            risk_level = decision.risk_level.as_str(),
            confidence = decision.confidence,
            fallback_policy = decision.fallback_policy.as_str(),
            tool_trust_class = decision.tool_trust_class.as_str(),
            reason = %decision.reason,
            policy_id = ?decision.policy_id,
            "omega route decision selected"
        );
    }

    pub(crate) fn record_shortcut_fallback(
        &self,
        session_id: &str,
        decision: &OmegaDecision,
        workflow_mode: WorkflowBridgeMode,
        tool_name: &str,
        action: ShortcutFallbackAction,
        error: &anyhow::Error,
    ) {
        tracing::warn!(
            event = SessionEvent::RouteFallbackApplied.as_str(),
            session_id,
            workflow_mode = workflow_mode.as_str(),
            tool_name,
            route = decision.route.as_str(),
            fallback_policy = decision.fallback_policy.as_str(),
            fallback_action = action.as_str(),
            error = %error,
            "omega route fallback applied"
        );
    }

    pub(crate) fn record_graph_plan(&self, session_id: &str, plan: &GraphExecutionPlan) {
        tracing::debug!(
            event = SessionEvent::RouteGraphPlanGenerated.as_str(),
            session_id,
            plan_id = %plan.plan_id,
            plan_version = %plan.plan_version,
            route = plan.route.as_str(),
            workflow_mode = plan.workflow_mode.as_str(),
            tool_name = %plan.tool_name,
            fallback_policy = plan.fallback_policy.as_str(),
            step_count = plan.steps.len(),
            "graph execution plan generated"
        );
    }

    pub(in crate::agent) fn record_graph_plan_step_started(
        &self,
        session_id: &str,
        plan: &GraphExecutionPlan,
        step: &crate::contracts::GraphPlanStep,
        attempt: u32,
    ) {
        tracing::debug!(
            event = SessionEvent::RouteGraphStepStarted.as_str(),
            session_id,
            plan_id = %plan.plan_id,
            plan_version = %plan.plan_version,
            step_index = step.index,
            step_id = %step.id,
            step_kind = ?step.kind,
            step_tool_name = step.tool_name.as_deref(),
            step_fallback_action = step.fallback_action.as_deref(),
            attempt,
            "graph plan step started"
        );
    }

    pub(in crate::agent) fn record_graph_plan_step_succeeded(
        &self,
        session_id: &str,
        plan: &GraphExecutionPlan,
        step: &crate::contracts::GraphPlanStep,
        attempt: u32,
        status: &str,
    ) {
        tracing::debug!(
            event = SessionEvent::RouteGraphStepSucceeded.as_str(),
            session_id,
            plan_id = %plan.plan_id,
            plan_version = %plan.plan_version,
            step_index = step.index,
            step_id = %step.id,
            step_kind = ?step.kind,
            step_tool_name = step.tool_name.as_deref(),
            step_fallback_action = step.fallback_action.as_deref(),
            attempt,
            status,
            "graph plan step succeeded"
        );
    }

    pub(in crate::agent) fn record_graph_plan_step_failed(
        &self,
        session_id: &str,
        plan: &GraphExecutionPlan,
        step: &crate::contracts::GraphPlanStep,
        attempt: u32,
        error: &anyhow::Error,
    ) {
        tracing::warn!(
            event = SessionEvent::RouteGraphStepFailed.as_str(),
            session_id,
            plan_id = %plan.plan_id,
            plan_version = %plan.plan_version,
            step_index = step.index,
            step_id = %step.id,
            step_kind = ?step.kind,
            step_tool_name = step.tool_name.as_deref(),
            step_fallback_action = step.fallback_action.as_deref(),
            attempt,
            error = %error,
            "graph plan step failed"
        );
    }

    pub(in crate::agent) fn record_graph_execution_completed(
        &self,
        session_id: &str,
        plan: &GraphExecutionPlan,
        tool_attempts: u32,
        output_chars: usize,
    ) {
        tracing::debug!(
            event = SessionEvent::RouteGraphExecutionCompleted.as_str(),
            session_id,
            plan_id = %plan.plan_id,
            plan_version = %plan.plan_version,
            route = plan.route.as_str(),
            workflow_mode = plan.workflow_mode.as_str(),
            tool_name = %plan.tool_name,
            tool_attempts,
            output_chars,
            "graph execution completed"
        );
    }

    pub(in crate::agent) fn record_graph_execution_rerouted(
        &self,
        session_id: &str,
        plan: &GraphExecutionPlan,
        target_route: &str,
        reason: &str,
    ) {
        tracing::info!(
            event = SessionEvent::RouteGraphExecutionRerouted.as_str(),
            session_id,
            plan_id = %plan.plan_id,
            plan_version = %plan.plan_version,
            route = plan.route.as_str(),
            workflow_mode = plan.workflow_mode.as_str(),
            tool_name = %plan.tool_name,
            target_route,
            reason,
            "graph execution rerouted"
        );
    }

    pub(in crate::agent) async fn record_route_trace(&self, trace: &RouteTrace) {
        let route_trace_json = serde_json::to_string(trace)
            .unwrap_or_else(|_| "{\"error\":\"route_trace_serialize_failed\"}".to_string());
        let fallback_applied = trace.fallback_applied.unwrap_or(false);
        let fallback_policy = trace
            .fallback_policy
            .map(OmegaFallbackPolicy::as_str)
            .unwrap_or("none");
        let workflow_mode = trace
            .workflow_mode
            .map(|mode| mode.as_str())
            .unwrap_or("none");
        let plan_id = trace.plan_id.as_deref().unwrap_or("");
        let failure_taxonomy_json =
            serde_json::to_string(&trace.failure_taxonomy).unwrap_or_else(|_| "[]".to_string());
        let graph_steps = trace.graph_steps.clone().unwrap_or_default();
        let graph_steps_json =
            serde_json::to_string(&graph_steps).unwrap_or_else(|_| "[]".to_string());
        let mut stream_fields = vec![
            (
                "kind".to_string(),
                SessionEvent::RouteTraceEmitted.as_str().to_string(),
            ),
            ("session_id".to_string(), trace.session_id.clone()),
            ("turn_id".to_string(), trace.turn_id.to_string()),
            (
                "selected_route".to_string(),
                trace.selected_route.as_str().to_string(),
            ),
            ("confidence".to_string(), format!("{:.6}", trace.confidence)),
            (
                "risk_level".to_string(),
                trace.risk_level.as_str().to_string(),
            ),
            (
                "tool_trust_class".to_string(),
                trace.tool_trust_class.as_str().to_string(),
            ),
            ("fallback_applied".to_string(), fallback_applied.to_string()),
            ("fallback_policy".to_string(), fallback_policy.to_string()),
            ("plan_id".to_string(), plan_id.to_string()),
            ("workflow_mode".to_string(), workflow_mode.to_string()),
            (
                "tool_chain_len".to_string(),
                trace.tool_chain.len().to_string(),
            ),
            (
                "failure_count".to_string(),
                trace.failure_taxonomy.len().to_string(),
            ),
            (
                "graph_steps_count".to_string(),
                trace.graph_steps.as_ref().map_or(0, Vec::len).to_string(),
            ),
            (
                "latency_ms".to_string(),
                format!("{:.3}", trace.latency_ms.unwrap_or(0.0)),
            ),
            ("failure_taxonomy_json".to_string(), failure_taxonomy_json),
            ("graph_steps_json".to_string(), graph_steps_json),
            ("route_trace_json".to_string(), route_trace_json.clone()),
        ];
        if let Some(injection) = trace.injection.as_ref() {
            stream_fields.extend([
                (
                    "injection_blocks_used".to_string(),
                    injection.blocks_used.to_string(),
                ),
                (
                    "injection_chars_injected".to_string(),
                    injection.chars_injected.to_string(),
                ),
                (
                    "injection_dropped_by_budget".to_string(),
                    injection.dropped_by_budget.to_string(),
                ),
            ]);
        }
        if let Err(error) = self
            .session
            .publish_stream_event(self.route_trace_stream_name(), stream_fields)
            .await
        {
            tracing::warn!(
                event = SessionEvent::RouteTraceEmitted.as_str(),
                session_id = %trace.session_id,
                turn_id = trace.turn_id,
                stream_name = self.route_trace_stream_name(),
                error = %error,
                "failed to publish route trace stream event"
            );
        }
        tracing::info!(
            event = SessionEvent::RouteTraceEmitted.as_str(),
            session_id = %trace.session_id,
            turn_id = trace.turn_id,
            selected_route = trace.selected_route.as_str(),
            confidence = trace.confidence,
            risk_level = trace.risk_level.as_str(),
            tool_trust_class = trace.tool_trust_class.as_str(),
            fallback_applied = trace.fallback_applied,
            fallback_policy = trace.fallback_policy.map(OmegaFallbackPolicy::as_str),
            plan_id = trace.plan_id.as_deref(),
            workflow_mode = trace.workflow_mode.map(|mode| mode.as_str()),
            tool_chain_len = trace.tool_chain.len(),
            failure_count = trace.failure_taxonomy.len(),
            graph_steps = trace.graph_steps.as_ref().map_or(0, Vec::len),
            latency_ms = trace.latency_ms,
            route_trace = %route_trace_json,
            "route trace emitted"
        );
    }

    pub(crate) fn record_injection_snapshot(&self, session_id: &str, snapshot: &InjectionSnapshot) {
        let role_mix_profile_id = snapshot
            .role_mix
            .as_ref()
            .map(|profile| profile.profile_id.as_str());
        let role_mix_roles = snapshot
            .role_mix
            .as_ref()
            .map_or(0, |profile| profile.roles.len());
        tracing::debug!(
            event = SessionEvent::InjectionSnapshotCreated.as_str(),
            session_id,
            snapshot_id = %snapshot.snapshot_id,
            turn_id = snapshot.turn_id,
            blocks = snapshot.blocks.len(),
            total_chars = snapshot.total_chars,
            dropped_blocks = snapshot.dropped_block_ids.len(),
            truncated_blocks = snapshot.truncated_block_ids.len(),
            role_mix_profile_id,
            role_mix_roles,
            "injection snapshot created"
        );
        for block_id in &snapshot.dropped_block_ids {
            tracing::debug!(
                event = SessionEvent::InjectionBlockDropped.as_str(),
                session_id,
                snapshot_id = %snapshot.snapshot_id,
                block_id,
                "injection block dropped"
            );
        }
        for block_id in &snapshot.truncated_block_ids {
            tracing::debug!(
                event = SessionEvent::InjectionBlockTruncated.as_str(),
                session_id,
                snapshot_id = %snapshot.snapshot_id,
                block_id,
                "injection block truncated"
            );
        }
    }

    pub(crate) async fn build_shortcut_injection_snapshot(
        &self,
        session_id: &str,
        turn_id: u64,
        user_message: &str,
    ) -> Result<Option<InjectionSnapshot>> {
        let mut context_messages = Vec::new();
        if let Some(ref w) = self.bounded_session {
            let summary_segments = w
                .get_recent_summary_segments(session_id, self.config.summary_max_segments)
                .await?;
            if !summary_segments.is_empty() {
                let segment_count = summary_segments.len();
                context_messages.extend(summary_segments.into_iter().enumerate().map(
                    |(index, segment)| ChatMessage {
                        role: "system".to_string(),
                        content: Some(format!(
                            "Compressed conversation history from older turns (segment {}/{}): {} (turns={}, tools={})",
                            index + 1,
                            segment_count,
                            segment.summary,
                            segment.turn_count,
                            segment.tool_calls
                        )),
                        tool_calls: None,
                        tool_call_id: None,
                        name: Some(context_budget::SESSION_SUMMARY_MESSAGE_NAME.to_string()),
                    },
                ));
            }
        }
        if let Some(snapshot) = self
            .inspect_session_system_prompt_injection(session_id)
            .await
        {
            context_messages.push(ChatMessage {
                role: "system".to_string(),
                content: Some(snapshot.xml),
                tool_calls: None,
                tool_call_id: None,
                name: Some(SYSTEM_PROMPT_INJECTION_CONTEXT_MESSAGE_NAME.to_string()),
            });
        }
        if let (Some(ref store), Some(ref mem_cfg)) =
            (self.memory_store.as_ref(), self.config.memory.as_ref())
        {
            let recall_plan = plan_memory_recall(MemoryRecallInput {
                base_k1: mem_cfg.recall_k1,
                base_k2: mem_cfg.recall_k2,
                base_lambda: mem_cfg.recall_lambda,
                context_budget_tokens: self.config.context_budget_tokens,
                context_budget_reserve_tokens: self.config.context_budget_reserve_tokens,
                context_tokens_before_recall: estimate_messages_tokens(&context_messages),
                active_turns_estimate: 0,
                window_max_turns: self.config.window_max_turns,
                summary_segment_count: 0,
            });
            let recall_feedback_bias = self.recall_feedback_bias(session_id).await;
            let recall_plan = apply_feedback_to_plan(recall_plan, recall_feedback_bias);
            let query_embedding = self
                .embedding_or_hash(user_message, store, mem_cfg.embedding_dim)
                .await;
            let recalled = store.two_phase_recall_with_embedding_for_scope(
                session_id,
                &query_embedding,
                recall_plan.k1,
                recall_plan.k2,
                recall_plan.lambda,
            );
            let recalled = filter_recalled_episodes(recalled, &recall_plan);
            if let Some(system_content) =
                build_memory_context_message(&recalled, recall_plan.max_context_chars)
            {
                context_messages.push(ChatMessage {
                    role: "system".to_string(),
                    content: Some(system_content),
                    tool_calls: None,
                    tool_call_id: None,
                    name: Some(MEMORY_RECALL_MESSAGE_NAME.to_string()),
                });
            }
        }
        if context_messages.is_empty() {
            return Ok(None);
        }
        let mut policy = InjectionPolicy::default();
        policy.max_chars = policy.max_chars.min(3_500);
        injection::build_snapshot_from_messages(session_id, turn_id, context_messages, policy)
            .map(Some)
            .context("failed to build shortcut injection snapshot")
    }
}
