#[allow(clippy::wildcard_imports)]
use super::*;

impl Agent {
    /// When window >= `consolidation_threshold_turns` and memory is enabled, drain oldest
    /// segment and store as episode.
    #[allow(clippy::too_many_lines)]
    pub(super) async fn try_consolidate(&self, session_id: &str) -> Result<()> {
        let (store, threshold, take, consolidate_async) = match (
            self.memory_store.clone(),
            self.config.consolidation_threshold_turns,
            self.config.consolidation_take_turns,
        ) {
            (Some(s), Some(t), take) if take > 0 => (s, t, take, self.config.consolidation_async),
            _ => return Ok(()),
        };
        let Some(ref w) = self.bounded_session else {
            return Ok(());
        };
        let started = Instant::now();
        let Some((turn_count, _total_tool_calls, _len)) = w.get_stats(session_id).await? else {
            return Ok(());
        };
        let turn_count = usize::try_from(turn_count).unwrap_or(usize::MAX);
        if turn_count < threshold {
            return Ok(());
        }
        let drained = w.drain_oldest_turns(session_id, take).await?;
        if drained.is_empty() {
            return Ok(());
        }
        let (intent, experience, outcome) = summarise_drained_turns(&drained);
        let drained_tool_calls: u32 = drained.iter().map(|(_, _, tools)| *tools).sum();
        let summary_text = build_consolidated_summary_text(&intent, &experience, &outcome);
        let summary_segment = SessionSummarySegment::new(
            summary_text,
            drained.len() / 2,
            drained_tool_calls,
            now_unix_ms(),
        );
        w.append_summary_segment(session_id, summary_segment)
            .await?;

        let id = format!("consolidated-{}-{}", session_id, now_unix_ms());
        let expected_dim = self
            .config
            .memory
            .as_ref()
            .map_or_else(|| store.encoder().dimension(), |cfg| cfg.embedding_dim);
        let embedding = match self.embedding_for_memory(&intent, expected_dim).await {
            Ok(embedding) => embedding,
            Err(error_kind) => {
                tracing::warn!(
                    event = SessionEvent::MemoryConsolidationStoreFailed.as_str(),
                    session_id,
                    reason = error_kind.as_str(),
                    "memory consolidation skipped due to embedding failure"
                );
                self.publish_memory_stream_event(vec![
                    (
                        "kind".to_string(),
                        "consolidation_skipped_embedding_failed".to_string(),
                    ),
                    ("session_id".to_string(), session_id.to_string()),
                    ("reason".to_string(), error_kind.as_str().to_string()),
                ])
                .await;
                return Ok(());
            }
        };
        let episode = Episode::new(id.clone(), intent, embedding, experience, outcome.clone());
        let reward = if outcome.to_lowercase().contains("error")
            || outcome.to_lowercase().contains("failed")
        {
            0.0
        } else {
            1.0
        };

        if consolidate_async {
            self.publish_memory_stream_event(vec![
                ("kind".to_string(), "consolidation_enqueued".to_string()),
                ("session_id".to_string(), session_id.to_string()),
                ("drained_turns".to_string(), (drained.len() / 2).to_string()),
                (
                    "drained_tool_calls".to_string(),
                    drained_tool_calls.to_string(),
                ),
                ("episode_id".to_string(), id.clone()),
            ])
            .await;
            let store_for_task = Arc::clone(&store);
            let id_for_task = id.clone();
            let session_id_for_task = session_id.to_string();
            let backend_for_task = self.memory_state_backend.clone();
            tokio::task::spawn_blocking(move || {
                match store_for_task.store_for_scope(&session_id_for_task, episode) {
                    Ok(_) => {
                        store_for_task.update_q(&id_for_task, reward);
                        persist_memory_state(
                            backend_for_task.as_ref(),
                            &store_for_task,
                            &session_id_for_task,
                            "consolidation",
                        );
                    }
                    Err(error) => {
                        tracing::warn!(
                            event = SessionEvent::MemoryConsolidationStoreFailed.as_str(),
                            session_id = %session_id_for_task,
                            error = %error,
                            "failed to store consolidated memory episode"
                        );
                    }
                }
            });
        } else {
            match store.store_for_scope(session_id, episode) {
                Ok(_) => {
                    store.update_q(&id, reward);
                    persist_memory_state(
                        self.memory_state_backend.as_ref(),
                        &store,
                        session_id,
                        "consolidation",
                    );
                    self.publish_memory_stream_event(vec![
                        ("kind".to_string(), "consolidation_stored".to_string()),
                        ("session_id".to_string(), session_id.to_string()),
                        ("episode_id".to_string(), id.clone()),
                        ("reward".to_string(), format!("{reward:.3}")),
                        ("drained_turns".to_string(), (drained.len() / 2).to_string()),
                        (
                            "drained_tool_calls".to_string(),
                            drained_tool_calls.to_string(),
                        ),
                    ])
                    .await;
                }
                Err(error) => {
                    tracing::warn!(
                        event = SessionEvent::MemoryConsolidationStoreFailed.as_str(),
                        session_id,
                        error = %error,
                        "failed to store consolidated memory episode"
                    );
                    self.publish_memory_stream_event(vec![
                        ("kind".to_string(), "consolidation_store_failed".to_string()),
                        ("session_id".to_string(), session_id.to_string()),
                        ("error".to_string(), error.to_string()),
                    ])
                    .await;
                }
            }
        }
        tracing::debug!(
            session_id,
            threshold,
            drained_turns = drained.len() / 2,
            drained_slots = drained.len(),
            drained_tool_calls,
            consolidate_async,
            duration_ms = started.elapsed().as_millis(),
            "session consolidation completed"
        );
        Ok(())
    }
}
