#[allow(clippy::wildcard_imports)]
use super::*;

impl Agent {
    pub(in crate::agent) async fn append_turn_to_session(
        &self,
        session_id: &str,
        user_msg: &str,
        assistant_msg: &str,
        tool_count: u32,
    ) -> Result<()> {
        if let Some(ref w) = self.bounded_session {
            w.append_turn(session_id, user_msg, assistant_msg, tool_count)
                .await?;
            self.try_consolidate(session_id).await?;
            self.try_store_turn(session_id, user_msg, assistant_msg, tool_count)
                .await;
            return Ok(());
        }
        let user = ChatMessage {
            role: "user".to_string(),
            content: Some(user_msg.to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        };
        let assistant = ChatMessage {
            role: "assistant".to_string(),
            content: Some(assistant_msg.to_string()),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        };
        self.session
            .append(session_id, vec![user, assistant])
            .await?;
        self.try_store_turn(session_id, user_msg, assistant_msg, tool_count)
            .await;
        Ok(())
    }

    /// When memory is enabled, store the current turn as one episode (intent=user, experience=assistant, outcome=completed/error).
    #[allow(clippy::too_many_lines)]
    async fn try_store_turn(
        &self,
        session_id: &str,
        user_msg: &str,
        assistant_msg: &str,
        tool_count: u32,
    ) {
        let Some(ref store) = self.memory_store else {
            return;
        };
        let outcome = classify_assistant_outcome(assistant_msg)
            .as_memory_label()
            .to_string();
        let reward = if outcome == "error" { 0.0 } else { 1.0 };
        let gate_policy = self.memory_gate_policy();
        let scope_key = Episode::normalize_scope(session_id);
        let normalized_intent = user_msg.trim();
        let existing_episode_id = store
            .get_all()
            .into_iter()
            .rev()
            .find(|episode| {
                episode.scope_key() == scope_key.as_str()
                    && episode.intent.trim() == normalized_intent
            })
            .map(|episode| episode.id);

        let (id, episode_source) = if let Some(existing_id) = existing_episode_id {
            (existing_id, "existing")
        } else {
            let expected_dim = self
                .config
                .memory
                .as_ref()
                .map_or_else(|| store.encoder().dimension(), |cfg| cfg.embedding_dim);
            let id = format!(
                "turn-{}-{}",
                session_id,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            );
            let embedding = match self.embedding_for_memory(user_msg, expected_dim).await {
                Ok(embedding) => embedding,
                Err(error_kind) => {
                    let fallback_embedding = store.encoder().encode(user_msg);
                    let fallback_dim = fallback_embedding.len();
                    let repaired_fallback = if fallback_dim == expected_dim {
                        fallback_embedding
                    } else {
                        super::super::embedding_dimension::repair_embedding_dimension(
                            &fallback_embedding,
                            expected_dim,
                        )
                    };

                    tracing::warn!(
                        event = SessionEvent::MemoryTurnStoreFailed.as_str(),
                        session_id,
                        reason = error_kind.as_str(),
                        tool_count,
                        fallback_strategy = "hash_encoder",
                        fallback_dim,
                        expected_dim,
                        "failed to build embedding for memory turn store; falling back to hash encoder"
                    );
                    self.publish_memory_stream_event(vec![
                        (
                            "kind".to_string(),
                            "turn_store_embedding_fallback_hash".to_string(),
                        ),
                        ("session_id".to_string(), session_id.to_string()),
                        ("reason".to_string(), error_kind.as_str().to_string()),
                        ("tool_count".to_string(), tool_count.to_string()),
                    ])
                    .await;
                    repaired_fallback
                }
            };
            let episode = Episode::new(
                id.clone(),
                user_msg.to_string(),
                embedding,
                assistant_msg.to_string(),
                outcome.clone(),
            );
            if let Err(error) = store.store_for_scope(session_id, episode) {
                tracing::warn!(
                    event = SessionEvent::MemoryTurnStoreFailed.as_str(),
                    session_id,
                    error = %error,
                    "failed to store memory episode for turn"
                );
                self.publish_memory_stream_event(vec![
                    ("kind".to_string(), "turn_store_failed".to_string()),
                    ("session_id".to_string(), session_id.to_string()),
                    ("error".to_string(), error.to_string()),
                ])
                .await;
                return;
            }
            (id, "new")
        };

        store.update_q(&id, reward);
        let _ = store.record_feedback(&id, reward > 0.0);

        if let Some(stored_episode) = store.get(&id) {
            let normalized_tool_count = u8::try_from(tool_count.min(6)).unwrap_or(6);
            let tool_count_f32 = f32::from(normalized_tool_count);
            let react_score = if reward > 0.0 {
                (0.72 + (tool_count_f32 * 0.04)).clamp(0.0, 1.0)
            } else {
                (0.20 + (tool_count_f32 * 0.01)).clamp(0.0, 1.0)
            };
            let graph_score = if tool_count > 0 { 0.64 } else { 0.45 };
            let omega_score = if reward > 0.0 { 0.78 } else { 0.22 };
            let ledger = MemoryUtilityLedger::from_episode(
                &stored_episode,
                react_score,
                graph_score,
                omega_score,
            );
            let decision = gate_policy.evaluate(
                &ledger,
                vec![
                    format!("react:tool_calls:{tool_count}"),
                    format!("react:outcome:{outcome}"),
                ],
                vec![format!("graph:turn_tool_count:{tool_count}")],
                vec![format!("omega:reward={reward:.3}")],
            );
            let gate_event = MemoryGateEvent::from_decision(
                session_id,
                self.next_runtime_turn_id(),
                &id,
                &ledger,
                decision.clone(),
            );
            tracing::debug!(
                event = SessionEvent::MemoryGateEvaluated.as_str(),
                session_id,
                episode_id = %id,
                episode_source,
                verdict = decision.verdict.as_str(),
                confidence = decision.confidence,
                ttl_score = gate_event.ttl_score,
                utility_score = ledger.utility_score,
                react_evidence_count = decision.react_evidence_refs.len(),
                graph_evidence_count = decision.graph_evidence_refs.len(),
                omega_factor_count = decision.omega_factors.len(),
                react_evidence_refs = ?decision.react_evidence_refs,
                graph_evidence_refs = ?decision.graph_evidence_refs,
                omega_factors = ?decision.omega_factors,
                next_action = %decision.next_action,
                reason = %decision.reason,
                "memory gate decision evaluated"
            );
            if matches!(decision.verdict, MemoryGateVerdict::Obsolete) && store.delete_episode(&id)
            {
                tracing::debug!(
                    event = SessionEvent::MemoryGateEvaluated.as_str(),
                    session_id,
                    episode_id = %id,
                    episode_source,
                    action = "purged",
                    "memory episode purged by gate decision"
                );
            }
            self.publish_memory_stream_event(vec![
                ("kind".to_string(), "memory_gate_event".to_string()),
                ("session_id".to_string(), session_id.to_string()),
                ("episode_id".to_string(), id.clone()),
                ("episode_source".to_string(), episode_source.to_string()),
                ("turn_id".to_string(), gate_event.turn_id.to_string()),
                (
                    "state_before".to_string(),
                    gate_event.state_before.as_str().to_string(),
                ),
                (
                    "state_after".to_string(),
                    gate_event.state_after.as_str().to_string(),
                ),
                (
                    "ttl_score".to_string(),
                    format!("{:.3}", gate_event.ttl_score),
                ),
                ("verdict".to_string(), decision.verdict.as_str().to_string()),
                (
                    "confidence".to_string(),
                    format!("{:.3}", decision.confidence),
                ),
                (
                    "react_evidence_count".to_string(),
                    decision.react_evidence_refs.len().to_string(),
                ),
                (
                    "graph_evidence_count".to_string(),
                    decision.graph_evidence_refs.len().to_string(),
                ),
                (
                    "omega_factor_count".to_string(),
                    decision.omega_factors.len().to_string(),
                ),
                (
                    "react_evidence_refs".to_string(),
                    encode_string_list_for_stream(&decision.react_evidence_refs),
                ),
                (
                    "graph_evidence_refs".to_string(),
                    encode_string_list_for_stream(&decision.graph_evidence_refs),
                ),
                (
                    "omega_factors".to_string(),
                    encode_string_list_for_stream(&decision.omega_factors),
                ),
                ("next_action".to_string(), decision.next_action.clone()),
            ])
            .await;

            if matches!(decision.verdict, MemoryGateVerdict::Promote) {
                tracing::info!(
                    event = SessionEvent::MemoryPromoted.as_str(),
                    session_id,
                    episode_id = %id,
                    episode_source,
                    confidence = decision.confidence,
                    next_action = %decision.next_action,
                    reason = %decision.reason,
                    "memory episode promoted and queued for durable knowledge ingestion"
                );
                self.publish_memory_stream_event(vec![
                    ("kind".to_string(), "memory_promoted".to_string()),
                    ("session_id".to_string(), session_id.to_string()),
                    ("episode_id".to_string(), id.clone()),
                    ("episode_source".to_string(), episode_source.to_string()),
                    (
                        "scope_key".to_string(),
                        stored_episode.scope_key().to_string(),
                    ),
                    ("turn_id".to_string(), gate_event.turn_id.to_string()),
                    ("verdict".to_string(), decision.verdict.as_str().to_string()),
                    (
                        "confidence".to_string(),
                        format!("{:.3}", decision.confidence),
                    ),
                    (
                        "utility_score".to_string(),
                        format!("{:.3}", ledger.utility_score),
                    ),
                    ("ttl_score".to_string(), format!("{:.3}", ledger.ttl_score)),
                    ("q_value".to_string(), format!("{:.3}", ledger.q_value)),
                    (
                        "failure_rate".to_string(),
                        format!("{:.3}", ledger.failure_rate),
                    ),
                    ("usage_count".to_string(), ledger.usage_count.to_string()),
                    (
                        "intent_excerpt".to_string(),
                        stream_excerpt(&stored_episode.intent, 512),
                    ),
                    (
                        "experience_excerpt".to_string(),
                        stream_excerpt(&stored_episode.experience, 1024),
                    ),
                    ("outcome".to_string(), stored_episode.outcome.clone()),
                    ("reason".to_string(), stream_excerpt(&decision.reason, 512)),
                    (
                        "react_evidence_refs".to_string(),
                        encode_string_list_for_stream(&decision.react_evidence_refs),
                    ),
                    (
                        "graph_evidence_refs".to_string(),
                        encode_string_list_for_stream(&decision.graph_evidence_refs),
                    ),
                    (
                        "omega_factors".to_string(),
                        encode_string_list_for_stream(&decision.omega_factors),
                    ),
                    ("next_action".to_string(), decision.next_action.clone()),
                    (
                        "knowledge_ingest_hint".to_string(),
                        "knowledge.ingest_candidate".to_string(),
                    ),
                ])
                .await;
            }
        }

        persist_memory_state(
            self.memory_state_backend.as_ref(),
            store,
            session_id,
            "turn_store",
        );
        self.maybe_apply_memory_decay(session_id, store);
        self.publish_memory_stream_event(vec![
            ("kind".to_string(), "turn_stored".to_string()),
            ("session_id".to_string(), session_id.to_string()),
            ("episode_id".to_string(), id),
            ("episode_source".to_string(), episode_source.to_string()),
            ("outcome".to_string(), outcome),
            ("reward".to_string(), format!("{reward:.3}")),
        ])
        .await;
    }
}
