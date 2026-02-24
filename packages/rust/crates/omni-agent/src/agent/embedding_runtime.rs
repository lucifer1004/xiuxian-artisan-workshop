use super::{
    Agent, EMBEDDING_SOURCE_EMBEDDING, EMBEDDING_SOURCE_EMBEDDING_REPAIRED, SessionEvent,
    repair_embedding_dimension,
};
use std::sync::atomic::Ordering;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) const EMBEDDING_SOURCE_UNAVAILABLE: &str = "embedding_unavailable";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemoryEmbeddingErrorKind {
    CooldownActive,
    Timeout,
    Unavailable,
}

impl MemoryEmbeddingErrorKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::CooldownActive => "cooldown_active",
            Self::Timeout => "timeout",
            Self::Unavailable => "unavailable",
        }
    }
}

impl Agent {
    async fn try_embed_intent(
        &self,
        intent: &str,
        expected_dim: usize,
    ) -> Option<(Vec<f32>, &'static str)> {
        let client = self.embedding_client.as_ref()?;
        let model = self
            .config
            .memory
            .as_ref()
            .and_then(|cfg| cfg.embedding_model.as_deref());
        let embedded = client.embed_with_model(intent, model).await?;
        if embedded.len() == expected_dim {
            return Some((embedded, EMBEDDING_SOURCE_EMBEDDING));
        }
        let repaired = repair_embedding_dimension(&embedded, expected_dim);
        tracing::warn!(
            event = SessionEvent::MemoryEmbeddingDimMismatch.as_str(),
            returned_dim = embedded.len(),
            expected_dim,
            repair_strategy = "resample",
            "embedding dimension mismatch; repaired vector for memory operations"
        );
        Some((repaired, EMBEDDING_SOURCE_EMBEDDING_REPAIRED))
    }

    pub(crate) async fn embedding_for_memory(
        &self,
        intent: &str,
        expected_dim: usize,
    ) -> std::result::Result<Vec<f32>, MemoryEmbeddingErrorKind> {
        self.embedding_for_memory_with_source(intent, expected_dim)
            .await
            .map(|(embedding, _)| embedding)
    }

    pub(crate) async fn embedding_for_memory_with_source(
        &self,
        intent: &str,
        expected_dim: usize,
    ) -> std::result::Result<(Vec<f32>, &'static str), MemoryEmbeddingErrorKind> {
        let cooldown_until = self
            .memory_embed_timeout_cooldown_until_ms
            .load(Ordering::Relaxed);
        let now_ms = current_unix_millis();
        if cooldown_until > now_ms {
            self.record_memory_embedding_cooldown_reject_metric().await;
            tracing::debug!(
                event = "agent.memory.embedding.cooldown_active",
                cooldown_remaining_ms = cooldown_until.saturating_sub(now_ms),
                cooldown_total_ms = duration_to_u64_millis(self.memory_embed_timeout_cooldown),
                "memory embedding timeout cooldown active; rejecting embedding request"
            );
            return Err(MemoryEmbeddingErrorKind::CooldownActive);
        }

        match tokio::time::timeout(
            self.memory_embed_timeout,
            self.try_embed_intent(intent, expected_dim),
        )
        .await
        {
            Ok(Some((embedding, source))) => {
                self.memory_embed_timeout_cooldown_until_ms
                    .store(0, Ordering::Relaxed);
                self.record_memory_embedding_success_metric().await;
                Ok((embedding, source))
            }
            Ok(None) => {
                self.record_memory_embedding_unavailable_metric().await;
                tracing::warn!(
                    event = "agent.memory.embedding.unavailable",
                    "memory embedding unavailable; semantic memory operation skipped"
                );
                Err(MemoryEmbeddingErrorKind::Unavailable)
            }
            Err(_) => {
                let cooldown_ms = duration_to_u64_millis(self.memory_embed_timeout_cooldown);
                if cooldown_ms > 0 {
                    self.memory_embed_timeout_cooldown_until_ms.store(
                        current_unix_millis().saturating_add(cooldown_ms),
                        Ordering::Relaxed,
                    );
                }
                self.record_memory_embedding_timeout_metric().await;
                tracing::warn!(
                    event = "agent.memory.embedding.timeout",
                    timeout_ms = self.memory_embed_timeout.as_millis(),
                    cooldown_ms,
                    "memory embedding timed out; semantic memory operation skipped"
                );
                Err(MemoryEmbeddingErrorKind::Timeout)
            }
        }
    }
}

fn current_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| u64::try_from(duration.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or_default()
}

fn duration_to_u64_millis(duration: std::time::Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}
