#[allow(clippy::wildcard_imports)]
use super::*;

impl Agent {
    #[allow(clippy::too_many_lines)]
    pub(in crate::agent) async fn run_react_loop(
        &self,
        session_id: &str,
        user_message: &str,
        force_react: bool,
        turn_id: u64,
    ) -> Result<String> {
        let policy_hint = self.take_reflection_policy_hint(session_id).await;
        if let Some(hint) = policy_hint.as_ref() {
            tracing::debug!(
                event = SessionEvent::ReflectionPolicyHintApplied.as_str(),
                session_id,
                source_turn_id = hint.source_turn_id,
                preferred_route = hint.preferred_route.as_str(),
                risk_floor = hint.risk_floor.as_str(),
                fallback_override = hint.fallback_override.map(OmegaFallbackPolicy::as_str),
                tool_trust_class = hint.tool_trust_class.as_str(),
                reason = %hint.reason,
                "reflection policy hint applied to route decision"
            );
        }
        let decision = omega::apply_quality_gate(omega::apply_policy_hint(
            omega::decide_for_standard_turn(force_react),
            policy_hint.as_ref(),
        ));
        self.record_omega_decision(session_id, &decision, None, None);

        let mut summary_segments: Vec<SessionSummarySegment> = Vec::new();
        let mut messages: Vec<ChatMessage> = if let Some(ref w) = self.bounded_session {
            let limit = self.config.window_max_turns.unwrap_or(512);
            summary_segments = w
                .get_recent_summary_segments(session_id, self.config.summary_max_segments)
                .await?;
            w.get_recent_messages(session_id, limit).await?
        } else {
            self.session.get(session_id).await?
        };

        if !summary_segments.is_empty() {
            let segment_count = summary_segments.len();
            let summary_messages = summary_segments
                .iter()
                .enumerate()
                .map(|(index, segment)| ChatMessage {
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
                })
                .collect::<Vec<_>>();
            messages.splice(0..0, summary_messages);
        }

        if let Some(snapshot) = self
            .inspect_session_system_prompt_injection(session_id)
            .await
        {
            messages.insert(
                0,
                ChatMessage {
                    role: "system".to_string(),
                    content: Some(snapshot.xml),
                    tool_calls: None,
                    tool_call_id: None,
                    name: Some(SYSTEM_PROMPT_INJECTION_CONTEXT_MESSAGE_NAME.to_string()),
                },
            );
        }

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: Some(user_message.to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        });

        let mut recall_credit_candidates: Vec<RecalledEpisodeCandidate> = Vec::new();

        if let (Some(store), Some(mem_cfg)) =
            (self.memory_store.as_ref(), self.config.memory.as_ref())
        {
            let recall_started = Instant::now();
            let active_turns_estimate = messages
                .iter()
                .filter(|message| message.role == "user" || message.role == "assistant")
                .count()
                / 2;
            let query_tokens = count_tokens(user_message);
            let recall_plan = plan_memory_recall(MemoryRecallInput {
                base_k1: mem_cfg.recall_k1,
                base_k2: mem_cfg.recall_k2,
                base_lambda: mem_cfg.recall_lambda,
                context_budget_tokens: self.config.context_budget_tokens,
                context_budget_reserve_tokens: self.config.context_budget_reserve_tokens,
                context_tokens_before_recall: estimate_messages_tokens(&messages),
                active_turns_estimate,
                window_max_turns: self.config.window_max_turns,
                summary_segment_count: summary_segments.len(),
            });
            let recall_feedback_bias = self.recall_feedback_bias(session_id).await;
            let recall_plan = apply_feedback_to_plan(recall_plan, recall_feedback_bias);
            tracing::debug!(
                event = SessionEvent::MemoryRecallPlanned.as_str(),
                session_id,
                memory_scope = session_id,
                k1 = recall_plan.k1,
                k2 = recall_plan.k2,
                lambda = recall_plan.lambda,
                min_score = recall_plan.min_score,
                max_context_chars = recall_plan.max_context_chars,
                budget_pressure = recall_plan.budget_pressure,
                window_pressure = recall_plan.window_pressure,
                effective_budget_tokens = ?recall_plan.effective_budget_tokens,
                active_turns_estimate,
                summary_segment_count = summary_segments.len(),
                recall_feedback_bias,
                "memory recall plan selected"
            );
            self.record_memory_recall_plan_metrics().await;

            match self
                .embedding_for_memory_with_source(user_message, mem_cfg.embedding_dim)
                .await
            {
                Ok((query_embedding, embedding_source)) => {
                    let recalled = store.two_phase_recall_with_embedding_for_scope(
                        session_id,
                        &query_embedding,
                        recall_plan.k1,
                        recall_plan.k2,
                        recall_plan.lambda,
                    );
                    let recalled_count = recalled.len();
                    let recalled = filter_recalled_episodes(recalled, &recall_plan);
                    if let Some(system_content) =
                        build_memory_context_message(&recalled, recall_plan.max_context_chars)
                    {
                        if mem_cfg.recall_credit_enabled {
                            recall_credit_candidates = select_recall_credit_candidates(
                                &recalled,
                                mem_cfg.recall_credit_max_candidates,
                            );
                        }
                        let injected_count = recalled.len();
                        let context_chars_injected = system_content.chars().count();
                        let pipeline_duration_ms =
                            u64::try_from(recall_started.elapsed().as_millis()).unwrap_or(u64::MAX);
                        let best_score = recalled
                            .first()
                            .map(|(_, score)| *score)
                            .unwrap_or_default();
                        let weakest_score =
                            recalled.last().map(|(_, score)| *score).unwrap_or_default();
                        messages.insert(
                            0,
                            ChatMessage {
                                role: "system".to_string(),
                                content: Some(system_content),
                                tool_calls: None,
                                tool_call_id: None,
                                name: Some(MEMORY_RECALL_MESSAGE_NAME.to_string()),
                            },
                        );
                        tracing::debug!(
                            event = SessionEvent::MemoryRecallInjected.as_str(),
                            session_id,
                            query_tokens,
                            embedding_source,
                            recalled_total = recalled_count,
                            recalled_selected = recalled.len(),
                            recalled_injected = injected_count,
                            context_chars_injected,
                            pipeline_duration_ms,
                            best_score,
                            weakest_score,
                            "memory recall context injected"
                        );
                        self.record_memory_recall_result_metrics(
                            memory_recall_state::SessionMemoryRecallDecision::Injected,
                            recalled.len(),
                            injected_count,
                            context_chars_injected,
                            pipeline_duration_ms,
                        )
                        .await;
                        self.record_memory_recall_snapshot(
                            session_id,
                            memory_recall_state::SessionMemoryRecallSnapshot::from_plan(
                                recall_plan,
                                active_turns_estimate,
                                summary_segments.len(),
                                query_tokens,
                                recall_feedback_bias,
                                embedding_source,
                                recalled_count,
                                recalled.len(),
                                injected_count,
                                context_chars_injected,
                                Some(best_score),
                                Some(weakest_score),
                                pipeline_duration_ms,
                                memory_recall_state::SessionMemoryRecallDecision::Injected,
                            ),
                        )
                        .await;
                    } else {
                        let pipeline_duration_ms =
                            u64::try_from(recall_started.elapsed().as_millis()).unwrap_or(u64::MAX);
                        let best_score = recalled
                            .first()
                            .map(|(_, score)| *score)
                            .unwrap_or_default();
                        tracing::debug!(
                            event = SessionEvent::MemoryRecallSkipped.as_str(),
                            session_id,
                            query_tokens,
                            embedding_source,
                            recalled_total = recalled_count,
                            recalled_selected = recalled.len(),
                            pipeline_duration_ms,
                            best_score,
                            "memory recall skipped after scoring/compaction filters"
                        );
                        self.record_memory_recall_result_metrics(
                            memory_recall_state::SessionMemoryRecallDecision::Skipped,
                            recalled.len(),
                            0,
                            0,
                            pipeline_duration_ms,
                        )
                        .await;
                        self.record_memory_recall_snapshot(
                            session_id,
                            memory_recall_state::SessionMemoryRecallSnapshot::from_plan(
                                recall_plan,
                                active_turns_estimate,
                                summary_segments.len(),
                                query_tokens,
                                recall_feedback_bias,
                                embedding_source,
                                recalled_count,
                                recalled.len(),
                                0,
                                0,
                                recalled.first().map(|(_, score)| *score),
                                recalled.last().map(|(_, score)| *score),
                                pipeline_duration_ms,
                                memory_recall_state::SessionMemoryRecallDecision::Skipped,
                            ),
                        )
                        .await;
                    }
                }
                Err(error_kind) => {
                    let pipeline_duration_ms =
                        u64::try_from(recall_started.elapsed().as_millis()).unwrap_or(u64::MAX);
                    tracing::warn!(
                        event = SessionEvent::MemoryRecallSkipped.as_str(),
                        session_id,
                        query_tokens,
                        reason = error_kind.as_str(),
                        pipeline_duration_ms,
                        "memory recall skipped because embedding request failed"
                    );
                    self.record_memory_recall_result_metrics(
                        memory_recall_state::SessionMemoryRecallDecision::Skipped,
                        0,
                        0,
                        0,
                        pipeline_duration_ms,
                    )
                    .await;
                    self.record_memory_recall_snapshot(
                        session_id,
                        memory_recall_state::SessionMemoryRecallSnapshot::from_plan(
                            recall_plan,
                            active_turns_estimate,
                            summary_segments.len(),
                            query_tokens,
                            recall_feedback_bias,
                            EMBEDDING_SOURCE_UNAVAILABLE,
                            0,
                            0,
                            0,
                            0,
                            None,
                            None,
                            pipeline_duration_ms,
                            memory_recall_state::SessionMemoryRecallDecision::Skipped,
                        ),
                    )
                    .await;
                }
            }
        }

        let raw_messages = messages;
        match injection::normalize_messages_with_snapshot(
            session_id,
            turn_id,
            raw_messages.clone(),
            InjectionPolicy::default(),
        ) {
            Ok(normalized) => {
                if let Some(snapshot) = normalized.snapshot.as_ref() {
                    self.record_injection_snapshot(session_id, snapshot);
                }
                messages = normalized.messages;
            }
            Err(error) => {
                tracing::warn!(
                    session_id,
                    error = %error,
                    "failed to normalize injection snapshot; context messages unchanged"
                );
                messages = raw_messages;
            }
        }

        if let Some(context_budget_tokens) = self.config.context_budget_tokens
            && context_budget_tokens > 0
        {
            let result = context_budget::prune_messages_for_token_budget_with_strategy(
                messages,
                context_budget_tokens,
                self.config.context_budget_reserve_tokens,
                self.config.context_budget_strategy,
            );
            messages = result.messages;
            let report = result.report;
            self.record_context_budget_snapshot(session_id, &report)
                .await;
            tracing::debug!(
                session_id,
                strategy = report.strategy.as_str(),
                budget_tokens = report.budget_tokens,
                reserve_tokens = report.reserve_tokens,
                effective_budget_tokens = report.effective_budget_tokens,
                pre_messages = report.pre_messages,
                post_messages = report.post_messages,
                pre_tokens = report.pre_tokens,
                post_tokens = report.post_tokens,
                dropped_messages = report.pre_messages.saturating_sub(report.post_messages),
                dropped_tokens = report.pre_tokens.saturating_sub(report.post_tokens),
                non_system_pre_messages = report.non_system.input_messages,
                non_system_kept_messages = report.non_system.kept_messages,
                non_system_dropped_messages = report.non_system.dropped_messages(),
                non_system_pre_tokens = report.non_system.input_tokens,
                non_system_kept_tokens = report.non_system.kept_tokens,
                non_system_dropped_tokens = report.non_system.dropped_tokens(),
                non_system_truncated_messages = report.non_system.truncated_messages,
                non_system_truncated_tokens = report.non_system.truncated_tokens,
                regular_system_pre_messages = report.regular_system.input_messages,
                regular_system_kept_messages = report.regular_system.kept_messages,
                regular_system_dropped_messages = report.regular_system.dropped_messages(),
                regular_system_pre_tokens = report.regular_system.input_tokens,
                regular_system_kept_tokens = report.regular_system.kept_tokens,
                regular_system_dropped_tokens = report.regular_system.dropped_tokens(),
                regular_system_truncated_messages = report.regular_system.truncated_messages,
                regular_system_truncated_tokens = report.regular_system.truncated_tokens,
                summary_pre_messages = report.summary_system.input_messages,
                summary_kept_messages = report.summary_system.kept_messages,
                summary_dropped_messages = report.summary_system.dropped_messages(),
                summary_pre_tokens = report.summary_system.input_tokens,
                summary_kept_tokens = report.summary_system.kept_tokens,
                summary_dropped_tokens = report.summary_system.dropped_tokens(),
                summary_truncated_messages = report.summary_system.truncated_messages,
                summary_truncated_tokens = report.summary_system.truncated_tokens,
                "applied token-budget context packing"
            );
        }

        let tools_json = if self.mcp.is_some() {
            self.mcp_tools_for_llm().await?
        } else {
            None
        };
        let mut tools_json = tools_json;

        let mut round = 0;
        let mut total_tool_calls_this_turn: u32 = 0;
        let mut last_tool_names: Vec<String> = Vec::new();
        let mut tool_summary = ToolExecutionSummary::default();
        loop {
            if round >= self.config.max_tool_rounds {
                let hint = format!(
                    "max_tool_rounds ({}) exceeded after {} rounds ({} tool calls). \
                    Try again with a fresh message (rounds reset per message), or increase \
                    OMNI_AGENT_MAX_TOOL_ROUNDS / telegram.max_tool_rounds. \
                    Last tools: {:?}",
                    self.config.max_tool_rounds, round, total_tool_calls_this_turn, last_tool_names
                );
                tracing::warn!("{}", hint);
                let outcome = self
                    .update_recall_feedback(session_id, user_message, &hint, Some(&tool_summary))
                    .await;
                self.apply_memory_recall_credit(session_id, &recall_credit_candidates, outcome);
                self.reflect_turn_and_update_policy_hint(
                    session_id,
                    turn_id,
                    decision.route,
                    user_message,
                    &hint,
                    "error",
                    total_tool_calls_this_turn,
                )
                .await;
                return Err(anyhow::anyhow!("{hint}"));
            }
            round += 1;

            let resp = match self.llm.chat(messages.clone(), tools_json.clone()).await {
                Ok(resp) => resp,
                Err(error) => {
                    if !is_context_window_exceeded_error(&error) {
                        return Err(error);
                    }

                    let error_text = format!("{error:#}");
                    let context_limit_hint = parse_context_window_limit_hint(&error_text);
                    tracing::warn!(
                        event = "agent.llm.context_window.repair.start",
                        session_id,
                        round,
                        context_limit_hint,
                        tools_enabled = tools_json.is_some(),
                        "llm context window exceeded; starting automatic context repair"
                    );

                    let mut recovered: Option<(
                        crate::llm::AssistantMessage,
                        Vec<ChatMessage>,
                        Option<Vec<serde_json::Value>>,
                    )> = None;
                    let mut last_context_error = error;

                    if tools_json.is_some() {
                        match self.llm.chat(messages.clone(), None).await {
                            Ok(resp) => {
                                tracing::warn!(
                                    event = "agent.llm.context_window.repair.success",
                                    session_id,
                                    round,
                                    strategy = "drop_tools_only",
                                    "llm context repair succeeded by dropping tools payload"
                                );
                                recovered = Some((resp, messages.clone(), None));
                            }
                            Err(retry_error) if is_context_window_exceeded_error(&retry_error) => {
                                last_context_error = retry_error;
                            }
                            Err(retry_error) => return Err(retry_error),
                        }
                    }

                    if recovered.is_none() {
                        for budget in context_window_recovery_budgets(context_limit_hint) {
                            let pruned =
                                context_budget::prune_messages_for_token_budget_with_strategy(
                                    messages.clone(),
                                    budget,
                                    0,
                                    self.config.context_budget_strategy,
                                )
                                .messages;
                            if pruned.is_empty() {
                                continue;
                            }

                            if let Some(ref tools_payload) = tools_json {
                                match self
                                    .llm
                                    .chat(pruned.clone(), Some(tools_payload.clone()))
                                    .await
                                {
                                    Ok(resp) => {
                                        tracing::warn!(
                                            event = "agent.llm.context_window.repair.success",
                                            session_id,
                                            round,
                                            strategy = "prune_keep_tools",
                                            repair_budget_tokens = budget,
                                            "llm context repair succeeded with pruned messages (tools kept)"
                                        );
                                        recovered =
                                            Some((resp, pruned.clone(), tools_json.clone()));
                                        break;
                                    }
                                    Err(retry_error)
                                        if is_context_window_exceeded_error(&retry_error) =>
                                    {
                                        last_context_error = retry_error;
                                    }
                                    Err(retry_error) => return Err(retry_error),
                                }
                            }

                            match self.llm.chat(pruned.clone(), None).await {
                                Ok(resp) => {
                                    tracing::warn!(
                                        event = "agent.llm.context_window.repair.success",
                                        session_id,
                                        round,
                                        strategy = "prune_drop_tools",
                                        repair_budget_tokens = budget,
                                        "llm context repair succeeded with pruned messages and tools disabled"
                                    );
                                    recovered = Some((resp, pruned, None));
                                    break;
                                }
                                Err(retry_error)
                                    if is_context_window_exceeded_error(&retry_error) =>
                                {
                                    last_context_error = retry_error;
                                }
                                Err(retry_error) => return Err(retry_error),
                            }
                        }
                    }

                    let Some((repaired_resp, repaired_messages, repaired_tools)) = recovered else {
                        tracing::error!(
                            event = "agent.llm.context_window.repair.failed",
                            session_id,
                            round,
                            context_limit_hint,
                            "llm context repair exhausted all retries"
                        );
                        return Err(last_context_error);
                    };

                    messages = repaired_messages;
                    tools_json = repaired_tools;
                    repaired_resp
                }
            };

            if let Some(ref tool_calls) = resp.tool_calls {
                if tool_calls.is_empty() {
                    let out = resp.content.unwrap_or_default();
                    let outcome = self
                        .update_recall_feedback(session_id, user_message, &out, Some(&tool_summary))
                        .await;
                    self.apply_memory_recall_credit(session_id, &recall_credit_candidates, outcome);
                    self.append_turn_to_session(
                        session_id,
                        user_message,
                        &out,
                        total_tool_calls_this_turn,
                    )
                    .await?;
                    self.reflect_turn_and_update_policy_hint(
                        session_id,
                        turn_id,
                        decision.route,
                        user_message,
                        &out,
                        "completed",
                        total_tool_calls_this_turn,
                    )
                    .await;
                    return Ok(out);
                }
                total_tool_calls_this_turn = total_tool_calls_this_turn
                    .saturating_add(u32::try_from(tool_calls.len()).unwrap_or(u32::MAX));
                last_tool_names = tool_calls
                    .iter()
                    .map(|tc| tc.function.name.clone())
                    .collect();
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: resp.content.clone(),
                    tool_calls: Some(tool_calls.clone()),
                    tool_call_id: None,
                    name: None,
                });
                for tc in tool_calls {
                    let name = tc.function.name.clone();
                    let args_str = tc.function.arguments.clone();
                    let args = if args_str.is_empty() {
                        None
                    } else {
                        serde_json::from_str(&args_str).ok()
                    };
                    let output = match self.call_mcp_tool_with_diagnostics(&name, args).await {
                        Ok(output) => {
                            tool_summary.record_result(output.is_error);
                            output
                        }
                        Err(error) => {
                            if let Some(soft_output) =
                                self.soft_fail_mcp_tool_error_output(&name, &error)
                            {
                                tool_summary.record_result(true);
                                soft_output
                            } else {
                                tool_summary.record_transport_failure();
                                let error_text = format!("tool `{name}` call failed: {error}");
                                let outcome = self
                                    .update_recall_feedback(
                                        session_id,
                                        user_message,
                                        &error_text,
                                        Some(&tool_summary),
                                    )
                                    .await;
                                self.apply_memory_recall_credit(
                                    session_id,
                                    &recall_credit_candidates,
                                    outcome,
                                );
                                self.reflect_turn_and_update_policy_hint(
                                    session_id,
                                    turn_id,
                                    decision.route,
                                    user_message,
                                    &error_text,
                                    "error",
                                    total_tool_calls_this_turn,
                                )
                                .await;
                                return Err(error);
                            }
                        }
                    };
                    messages.push(ChatMessage {
                        role: "tool".to_string(),
                        content: Some(output.text),
                        tool_calls: None,
                        tool_call_id: Some(tc.id.clone()),
                        name: Some(name),
                    });
                }
                continue;
            }

            let out = resp.content.unwrap_or_default();
            let outcome = self
                .update_recall_feedback(session_id, user_message, &out, Some(&tool_summary))
                .await;
            self.apply_memory_recall_credit(session_id, &recall_credit_candidates, outcome);
            self.append_turn_to_session(session_id, user_message, &out, total_tool_calls_this_turn)
                .await?;
            self.reflect_turn_and_update_policy_hint(
                session_id,
                turn_id,
                decision.route,
                user_message,
                &out,
                "completed",
                total_tool_calls_this_turn,
            )
            .await;
            return Ok(out);
        }
    }
}

fn is_context_window_exceeded_error(error: &anyhow::Error) -> bool {
    let lower = format!("{error:#}").to_ascii_lowercase();
    lower.contains("context window exceeds limit")
        || lower.contains("maximum context length")
        || lower.contains("context_length_exceeded")
        || lower.contains("prompt is too long")
        || lower.contains("context limit")
}

fn parse_context_window_limit_hint(error_text: &str) -> Option<usize> {
    let lower = error_text.to_ascii_lowercase();
    let mut cursor = 0usize;
    while let Some(offset) = lower[cursor..].find("limit") {
        let start = cursor + offset + "limit".len();
        let tail = &lower[start..];
        let digits: String = tail
            .chars()
            .skip_while(|ch| !ch.is_ascii_digit())
            .take_while(char::is_ascii_digit)
            .collect();
        if let Ok(value) = digits.parse::<usize>() {
            return Some(value);
        }
        cursor = start;
    }
    None
}

fn context_window_recovery_budgets(limit_hint: Option<usize>) -> Vec<usize> {
    let mut budgets = if let Some(limit) = limit_hint {
        vec![
            limit.saturating_mul(3) / 5,
            limit.saturating_mul(1) / 2,
            limit.saturating_mul(2) / 5,
            limit.saturating_mul(1) / 3,
            limit.saturating_mul(1) / 4,
        ]
    } else {
        vec![1024, 768, 512, 384, 256]
    };
    budgets.retain(|budget| *budget > 0);
    budgets.sort_unstable();
    budgets.dedup();
    budgets.reverse();
    budgets
}
