//! Memory-recall helpers exposed for integration tests.

use xiuxian_memory_engine::Episode;

use crate::agent::memory_recall;

/// Test-facing memory recall input options.
#[derive(Debug, Clone, Copy)]
pub struct MemoryRecallInput {
    /// Base initial candidate count.
    pub base_k1: usize,
    /// Base final selected candidate count.
    pub base_k2: usize,
    /// Base relevance/recency fusion weight.
    pub base_lambda: f32,
    /// Optional context budget token cap.
    pub context_budget_tokens: Option<usize>,
    /// Reserved tokens not usable for recall context.
    pub context_budget_reserve_tokens: usize,
    /// Estimated token count already consumed before recall injection.
    pub context_tokens_before_recall: usize,
    /// Estimated active turn count for current session.
    pub active_turns_estimate: usize,
    /// Optional bounded-window max turns.
    pub window_max_turns: Option<usize>,
    /// Number of summary segments already present.
    pub summary_segment_count: usize,
}

/// Test-facing memory recall execution plan.
#[derive(Debug, Clone, Copy)]
pub struct MemoryRecallPlan {
    /// Candidate count before scoring cutdown.
    pub k1: usize,
    /// Candidate count after final selection.
    pub k2: usize,
    /// Effective relevance/recency fusion weight.
    pub lambda: f32,
    /// Minimum score threshold for selection.
    pub min_score: f32,
    /// Maximum generated context characters.
    pub max_context_chars: usize,
    /// Budget pressure factor used in planning.
    pub budget_pressure: f32,
    /// Window pressure factor used in planning.
    pub window_pressure: f32,
    /// Effective budget tokens used for planning.
    pub effective_budget_tokens: Option<usize>,
}

/// Build a memory-recall execution plan.
#[must_use]
pub fn plan_memory_recall(input: MemoryRecallInput) -> MemoryRecallPlan {
    let internal = memory_recall::plan_memory_recall(to_internal_input(input));
    from_internal_plan(internal)
}

/// Filter recalled episodes using the supplied plan and current time.
#[must_use]
pub fn filter_recalled_episodes(
    recalled: Vec<(Episode, f32)>,
    plan: &MemoryRecallPlan,
) -> Vec<(Episode, f32)> {
    let internal_plan = to_internal_plan(*plan);
    memory_recall::filter_recalled_episodes(recalled, &internal_plan)
}

/// Filter recalled episodes using an explicit timestamp for deterministic tests.
#[must_use]
pub fn filter_recalled_episodes_at(
    recalled: Vec<(Episode, f32)>,
    plan: &MemoryRecallPlan,
    now_unix_ms: i64,
) -> Vec<(Episode, f32)> {
    let internal_plan = to_internal_plan(*plan);
    memory_recall::filter_recalled_episodes_at(recalled, &internal_plan, now_unix_ms)
}

/// Build a memory context message under a character budget.
#[must_use]
pub fn build_memory_context_message(
    recalled: &[(Episode, f32)],
    max_chars: usize,
) -> Option<String> {
    memory_recall::build_memory_context_message(recalled, max_chars)
}

fn to_internal_input(input: MemoryRecallInput) -> memory_recall::MemoryRecallInput {
    memory_recall::MemoryRecallInput {
        base_k1: input.base_k1,
        base_k2: input.base_k2,
        base_lambda: input.base_lambda,
        context_budget_tokens: input.context_budget_tokens,
        context_budget_reserve_tokens: input.context_budget_reserve_tokens,
        context_tokens_before_recall: input.context_tokens_before_recall,
        active_turns_estimate: input.active_turns_estimate,
        window_max_turns: input.window_max_turns,
        summary_segment_count: input.summary_segment_count,
    }
}

fn to_internal_plan(plan: MemoryRecallPlan) -> memory_recall::MemoryRecallPlan {
    memory_recall::MemoryRecallPlan {
        k1: plan.k1,
        k2: plan.k2,
        lambda: plan.lambda,
        min_score: plan.min_score,
        max_context_chars: plan.max_context_chars,
        budget_pressure: plan.budget_pressure,
        window_pressure: plan.window_pressure,
        effective_budget_tokens: plan.effective_budget_tokens,
    }
}

fn from_internal_plan(plan: memory_recall::MemoryRecallPlan) -> MemoryRecallPlan {
    MemoryRecallPlan {
        k1: plan.k1,
        k2: plan.k2,
        lambda: plan.lambda,
        min_score: plan.min_score,
        max_context_chars: plan.max_context_chars,
        budget_pressure: plan.budget_pressure,
        window_pressure: plan.window_pressure,
        effective_budget_tokens: plan.effective_budget_tokens,
    }
}
