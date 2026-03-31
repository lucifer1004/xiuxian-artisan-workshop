//! Memory decay and recall-credit helpers exposed for integration tests.

use xiuxian_memory_engine::{Episode, EpisodeStore};

use crate::agent::memory::{
    RecallCreditUpdate as InternalRecallCreditUpdate,
    RecalledEpisodeCandidate as InternalRecalledEpisodeCandidate,
    apply_recall_credit as apply_recall_credit_internal,
    sanitize_decay_factor as sanitize_decay_factor_internal,
    select_recall_credit_candidates as select_recall_credit_candidates_internal,
    should_apply_decay as should_apply_decay_internal,
};
use crate::agent::memory_recall_feedback::RecallOutcome as InternalRecallOutcome;

/// Test-facing recall feedback outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecallOutcome {
    Success,
    Failure,
}

/// Test-facing recalled candidate descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct RecalledEpisodeCandidate {
    pub episode_id: String,
    pub score: f32,
}

/// Test-facing recall credit update descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct RecallCreditUpdate {
    pub episode_id: String,
    pub score: f32,
    pub weight: f32,
    pub previous_q: f32,
    pub effective_reward: f32,
    pub updated_q: f32,
}

/// Decide whether memory decay should execute on this turn.
#[must_use]
pub fn should_apply_decay(decay_enabled: bool, decay_every_turns: usize, turn_index: u64) -> bool {
    should_apply_decay_internal(decay_enabled, decay_every_turns, turn_index)
}

/// Clamp/sanitize decay factor into supported numeric range.
#[must_use]
pub fn sanitize_decay_factor(raw: f32) -> f32 {
    sanitize_decay_factor_internal(raw)
}

/// Select ranked recall-credit candidates.
#[must_use]
pub fn select_recall_credit_candidates(
    recalled: &[(Episode, f32)],
    max_candidates: usize,
) -> Vec<RecalledEpisodeCandidate> {
    select_recall_credit_candidates_internal(recalled, max_candidates)
        .into_iter()
        .map(from_internal_candidate)
        .collect()
}

/// Apply recall credit updates to episode Q-table and feedback counters.
#[must_use]
pub fn apply_recall_credit(
    store: &EpisodeStore,
    candidates: &[RecalledEpisodeCandidate],
    outcome: RecallOutcome,
) -> Vec<RecallCreditUpdate> {
    let internal_candidates = candidates
        .iter()
        .map(|candidate| InternalRecalledEpisodeCandidate {
            episode_id: candidate.episode_id.clone(),
            score: candidate.score,
        })
        .collect::<Vec<_>>();
    let internal_outcome = match outcome {
        RecallOutcome::Success => InternalRecallOutcome::Success,
        RecallOutcome::Failure => InternalRecallOutcome::Failure,
    };
    apply_recall_credit_internal(store, &internal_candidates, internal_outcome)
        .into_iter()
        .map(from_internal_update)
        .collect()
}

fn from_internal_candidate(
    candidate: InternalRecalledEpisodeCandidate,
) -> RecalledEpisodeCandidate {
    RecalledEpisodeCandidate {
        episode_id: candidate.episode_id,
        score: candidate.score,
    }
}

fn from_internal_update(update: InternalRecallCreditUpdate) -> RecallCreditUpdate {
    RecallCreditUpdate {
        episode_id: update.episode_id,
        score: update.score,
        weight: update.weight,
        previous_q: update.previous_q,
        effective_reward: update.effective_reward,
        updated_q: update.updated_q,
    }
}
