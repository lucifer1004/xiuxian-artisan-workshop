use std::time::Duration;

use crate::analyzers::{RegisteredRepository, RepoIntelligenceError};

pub(super) const REPO_INDEX_ANALYSIS_TIMEOUT: Duration = Duration::from_secs(45);

fn bounded_usize_to_f64(value: usize) -> f64 {
    f64::from(u32::try_from(value).unwrap_or(u32::MAX))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RepoIndexTaskPriority {
    Background,
    Interactive,
}

#[derive(Debug, Clone)]
pub(super) struct RepoIndexTask {
    pub(super) repository: RegisteredRepository,
    pub(super) refresh: bool,
    pub(super) fingerprint: String,
    pub(super) priority: RepoIndexTaskPriority,
}

#[derive(Debug)]
pub(super) struct AdaptiveConcurrencyController {
    pub(super) current_limit: usize,
    pub(super) max_limit: usize,
    pub(super) success_streak: usize,
    pub(super) ema_elapsed_ms: Option<f64>,
    pub(super) baseline_elapsed_ms: Option<f64>,
    pub(super) previous_efficiency: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct AdaptiveConcurrencySnapshot {
    pub(super) current_limit: usize,
    pub(super) max_limit: usize,
}

impl AdaptiveConcurrencyController {
    pub(super) fn new() -> Self {
        let max_limit = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(1)
            .max(1);
        Self {
            current_limit: 1,
            max_limit,
            success_streak: 0,
            ema_elapsed_ms: None,
            baseline_elapsed_ms: None,
            previous_efficiency: None,
        }
    }

    #[cfg(test)]
    pub(super) fn new_for_test(max_limit: usize) -> Self {
        Self {
            current_limit: 1,
            max_limit: max_limit.max(1),
            success_streak: 0,
            ema_elapsed_ms: None,
            baseline_elapsed_ms: None,
            previous_efficiency: None,
        }
    }

    pub(super) fn snapshot(&self) -> AdaptiveConcurrencySnapshot {
        AdaptiveConcurrencySnapshot {
            current_limit: self.current_limit.max(1).min(self.max_limit.max(1)),
            max_limit: self.max_limit.max(1),
        }
    }

    pub(super) fn target_limit(&mut self, queued: usize, active: usize) -> usize {
        let demand = queued.saturating_add(active);
        if demand <= 1 {
            self.current_limit = 1;
            return 1;
        }
        if queued == 0 && active < self.current_limit {
            self.current_limit = active.max(1);
        }
        self.current_limit
            .max(1)
            .min(self.max_limit.max(1))
            .min(demand)
    }

    pub(super) fn record_success(&mut self, elapsed: Duration, queued_remaining: usize) {
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
        let baseline = self.ema_elapsed_ms.unwrap_or(elapsed_ms);
        self.ema_elapsed_ms = Some(if self.ema_elapsed_ms.is_some() {
            baseline.mul_add(0.75, elapsed_ms * 0.25)
        } else {
            elapsed_ms
        });
        let ema_elapsed_ms = self.ema_elapsed_ms.unwrap_or(elapsed_ms);
        self.baseline_elapsed_ms = Some(
            self.baseline_elapsed_ms
                .map_or(ema_elapsed_ms, |existing| existing.min(ema_elapsed_ms)),
        );

        let efficiency = bounded_usize_to_f64(self.current_limit) / ema_elapsed_ms.max(1.0);
        let previous_efficiency = self.previous_efficiency.unwrap_or(efficiency);
        let efficiency_ratio = if previous_efficiency > 0.0 {
            efficiency / previous_efficiency
        } else {
            1.0
        };
        let io_pressure_detected = self
            .baseline_elapsed_ms
            .is_some_and(|baseline_ms| ema_elapsed_ms >= baseline_ms * 3.0);

        if queued_remaining == 0 {
            self.success_streak = 0;
            self.previous_efficiency = Some(efficiency);
            return;
        }

        if io_pressure_detected || efficiency_ratio < 0.80 {
            self.current_limit = (self.current_limit / 2).max(1);
            self.success_streak = 0;
            self.previous_efficiency = Some(efficiency);
            return;
        }

        if efficiency_ratio >= 0.95 {
            self.success_streak = self.success_streak.saturating_add(1);
            if self.success_streak >= self.current_limit && self.current_limit < self.max_limit {
                self.current_limit += 1;
                self.success_streak = 0;
            }
            self.previous_efficiency = Some(efficiency);
            return;
        }

        self.success_streak = 0;
        self.previous_efficiency = Some(efficiency);
    }

    pub(super) fn record_failure(&mut self) {
        self.current_limit = (self.current_limit / 2).max(1);
        self.success_streak = 0;
        self.previous_efficiency = None;
    }
}

#[derive(Debug)]
pub(super) enum RepoTaskOutcome {
    Success {
        revision: Option<String>,
    },
    Failure {
        revision: Option<String>,
        error: RepoIntelligenceError,
    },
    Skipped,
}

#[derive(Debug)]
pub(super) struct RepoTaskFeedback {
    pub(super) repo_id: String,
    pub(super) elapsed: Duration,
    pub(super) outcome: RepoTaskOutcome,
}
