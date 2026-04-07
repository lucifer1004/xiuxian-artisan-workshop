use std::time::Duration;

fn bounded_usize_to_f64(value: usize) -> f64 {
    f64::from(u32::try_from(value).unwrap_or(u32::MAX))
}

fn rounded_f64_to_u64(value: f64) -> u64 {
    if !value.is_finite() {
        return 0;
    }
    let rounded = value.round();
    if rounded <= 0.0 {
        return 0;
    }
    rounded.to_string().parse::<u64>().unwrap_or(u64::MAX)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdaptiveConcurrencyAdjustment {
    Initialized,
    Expanded,
    Stable,
    IdleReset,
    ObservedIoPressure,
    ContractedIoPressure,
    ContractedEfficiencyDrop,
    ContractedFailure,
}

impl AdaptiveConcurrencyAdjustment {
    #[cfg(feature = "performance")]
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Initialized => "initialized",
            Self::Expanded => "expanded",
            Self::Stable => "stable",
            Self::IdleReset => "idle_reset",
            Self::ObservedIoPressure => "observed_io_pressure",
            Self::ContractedIoPressure => "contracted_io_pressure",
            Self::ContractedEfficiencyDrop => "contracted_efficiency_drop",
            Self::ContractedFailure => "contracted_failure",
        }
    }
}

#[derive(Debug)]
pub(crate) struct AdaptiveConcurrencyController {
    pub(crate) current_limit: usize,
    pub(crate) max_limit: usize,
    pub(crate) success_streak: usize,
    pub(crate) ema_elapsed_ms: Option<f64>,
    pub(crate) baseline_elapsed_ms: Option<f64>,
    pub(crate) previous_efficiency: Option<f64>,
    pub(crate) reference_limit: usize,
    pub(crate) io_pressure_streak: usize,
    pub(crate) last_elapsed_ms: Option<u64>,
    pub(crate) last_efficiency_ratio_pct: Option<u64>,
    pub(crate) last_adjustment: AdaptiveConcurrencyAdjustment,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct AdaptiveConcurrencySnapshot {
    pub(crate) current_limit: usize,
    pub(crate) max_limit: usize,
}

#[cfg(feature = "performance")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AdaptiveConcurrencyDebugSnapshot {
    pub(crate) current_limit: usize,
    pub(crate) max_limit: usize,
    pub(crate) success_streak: usize,
    pub(crate) reference_limit: usize,
    pub(crate) io_pressure_streak: usize,
    pub(crate) ema_elapsed_ms: Option<u64>,
    pub(crate) baseline_elapsed_ms: Option<u64>,
    pub(crate) last_elapsed_ms: Option<u64>,
    pub(crate) last_efficiency_ratio_pct: Option<u64>,
    pub(crate) last_adjustment: AdaptiveConcurrencyAdjustment,
}

impl AdaptiveConcurrencyController {
    pub(crate) fn new() -> Self {
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
            reference_limit: 1,
            io_pressure_streak: 0,
            last_elapsed_ms: None,
            last_efficiency_ratio_pct: None,
            last_adjustment: AdaptiveConcurrencyAdjustment::Initialized,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(max_limit: usize) -> Self {
        Self {
            current_limit: 1,
            max_limit: max_limit.max(1),
            success_streak: 0,
            ema_elapsed_ms: None,
            baseline_elapsed_ms: None,
            previous_efficiency: None,
            reference_limit: 1,
            io_pressure_streak: 0,
            last_elapsed_ms: None,
            last_efficiency_ratio_pct: None,
            last_adjustment: AdaptiveConcurrencyAdjustment::Initialized,
        }
    }

    pub(crate) fn snapshot(&self) -> AdaptiveConcurrencySnapshot {
        AdaptiveConcurrencySnapshot {
            current_limit: self.current_limit.max(1).min(self.max_limit.max(1)),
            max_limit: self.max_limit.max(1),
        }
    }

    #[cfg(feature = "performance")]
    pub(crate) fn debug_snapshot(&self) -> AdaptiveConcurrencyDebugSnapshot {
        AdaptiveConcurrencyDebugSnapshot {
            current_limit: self.current_limit.max(1).min(self.max_limit.max(1)),
            max_limit: self.max_limit.max(1),
            success_streak: self.success_streak,
            reference_limit: self.reference_limit,
            io_pressure_streak: self.io_pressure_streak,
            ema_elapsed_ms: self.ema_elapsed_ms.map(rounded_f64_to_u64),
            baseline_elapsed_ms: self.baseline_elapsed_ms.map(rounded_f64_to_u64),
            last_elapsed_ms: self.last_elapsed_ms,
            last_efficiency_ratio_pct: self.last_efficiency_ratio_pct,
            last_adjustment: self.last_adjustment,
        }
    }

    pub(crate) fn target_limit(&mut self, queued: usize, active: usize) -> usize {
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

    pub(crate) fn record_success(&mut self, elapsed: Duration, queued_remaining: usize) {
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
        self.last_elapsed_ms = Some(rounded_f64_to_u64(elapsed_ms));
        if self.reference_limit == self.current_limit {
            let previous_ema = self.ema_elapsed_ms.unwrap_or(elapsed_ms);
            let ema_elapsed_ms = if self.ema_elapsed_ms.is_some() {
                previous_ema.mul_add(0.75, elapsed_ms * 0.25)
            } else {
                elapsed_ms
            };
            self.ema_elapsed_ms = Some(ema_elapsed_ms);
            self.baseline_elapsed_ms =
                Some(self.baseline_elapsed_ms.map_or(ema_elapsed_ms, |existing| {
                    if ema_elapsed_ms <= existing {
                        existing.mul_add(0.75, ema_elapsed_ms * 0.25)
                    } else {
                        existing.mul_add(0.90, ema_elapsed_ms * 0.10)
                    }
                }));
        } else {
            self.reference_limit = self.current_limit;
            self.success_streak = 0;
            self.io_pressure_streak = 0;
            self.ema_elapsed_ms = Some(elapsed_ms);
            self.baseline_elapsed_ms = Some(elapsed_ms);
            self.previous_efficiency = None;
        }

        let ema_elapsed_ms = self.ema_elapsed_ms.unwrap_or(elapsed_ms);

        let efficiency = bounded_usize_to_f64(self.current_limit) / ema_elapsed_ms.max(1.0);
        let previous_efficiency = self.previous_efficiency.unwrap_or(efficiency);
        let efficiency_ratio = if previous_efficiency > 0.0 {
            efficiency / previous_efficiency
        } else {
            1.0
        };
        self.last_efficiency_ratio_pct = Some(rounded_f64_to_u64(efficiency_ratio * 100.0));
        let io_pressure_detected = self
            .baseline_elapsed_ms
            .is_some_and(|baseline_ms| ema_elapsed_ms >= baseline_ms * 3.0);

        if queued_remaining == 0 {
            self.success_streak = 0;
            self.io_pressure_streak = 0;
            self.previous_efficiency = Some(efficiency);
            self.last_adjustment = AdaptiveConcurrencyAdjustment::IdleReset;
            return;
        }

        if io_pressure_detected {
            self.io_pressure_streak = self.io_pressure_streak.saturating_add(1);
            if self.io_pressure_streak >= 2 {
                self.current_limit = (self.current_limit / 2).max(1);
                self.success_streak = 0;
                self.previous_efficiency = None;
                self.reference_limit = self.current_limit;
                self.ema_elapsed_ms = None;
                self.baseline_elapsed_ms = None;
                self.io_pressure_streak = 0;
                self.last_adjustment = AdaptiveConcurrencyAdjustment::ContractedIoPressure;
            } else {
                self.previous_efficiency = Some(efficiency);
                self.last_adjustment = AdaptiveConcurrencyAdjustment::ObservedIoPressure;
            }
            return;
        }
        self.io_pressure_streak = 0;

        if efficiency_ratio < 0.80 {
            self.current_limit = (self.current_limit / 2).max(1);
            self.success_streak = 0;
            self.previous_efficiency = None;
            self.reference_limit = self.current_limit;
            self.ema_elapsed_ms = None;
            self.baseline_elapsed_ms = None;
            self.last_adjustment = AdaptiveConcurrencyAdjustment::ContractedEfficiencyDrop;
            return;
        }

        if efficiency_ratio >= 0.95 {
            self.success_streak = self.success_streak.saturating_add(1);
            if self.success_streak >= self.current_limit && self.current_limit < self.max_limit {
                self.current_limit += 1;
                self.success_streak = 0;
                self.last_adjustment = AdaptiveConcurrencyAdjustment::Expanded;
            } else {
                self.last_adjustment = AdaptiveConcurrencyAdjustment::Stable;
            }
            self.previous_efficiency = Some(efficiency);
            return;
        }

        self.success_streak = 0;
        self.previous_efficiency = Some(efficiency);
        self.last_adjustment = AdaptiveConcurrencyAdjustment::Stable;
    }

    pub(crate) fn record_failure(&mut self) {
        self.current_limit = (self.current_limit / 2).max(1);
        self.success_streak = 0;
        self.reference_limit = self.current_limit;
        self.io_pressure_streak = 0;
        self.ema_elapsed_ms = None;
        self.baseline_elapsed_ms = None;
        self.previous_efficiency = None;
        self.last_elapsed_ms = None;
        self.last_efficiency_ratio_pct = None;
        self.last_adjustment = AdaptiveConcurrencyAdjustment::ContractedFailure;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{AdaptiveConcurrencyAdjustment, AdaptiveConcurrencyController};

    #[test]
    fn debug_snapshot_marks_io_pressure_contractions() {
        let mut controller = AdaptiveConcurrencyController::new_for_test(8);
        controller.current_limit = 6;
        controller.reference_limit = 6;
        controller.ema_elapsed_ms = Some(100.0);
        controller.baseline_elapsed_ms = Some(100.0);
        controller.previous_efficiency = Some(6.0 / 100.0);
        controller.io_pressure_streak = 1;

        controller.record_success(Duration::from_millis(450), 12);

        assert_eq!(
            controller.last_adjustment,
            AdaptiveConcurrencyAdjustment::ContractedIoPressure
        );
        assert_eq!(controller.current_limit, 3);
        assert_eq!(controller.last_elapsed_ms, Some(450));
        assert_eq!(controller.io_pressure_streak, 0);
    }

    #[test]
    fn debug_snapshot_marks_failure_contractions() {
        let mut controller = AdaptiveConcurrencyController::new_for_test(8);
        controller.current_limit = 4;

        controller.record_failure();

        assert_eq!(
            controller.last_adjustment,
            AdaptiveConcurrencyAdjustment::ContractedFailure
        );
        assert_eq!(controller.current_limit, 2);
    }

    #[test]
    fn debug_snapshot_resets_baseline_when_limit_changes() {
        let mut controller = AdaptiveConcurrencyController::new_for_test(8);
        controller.current_limit = 4;
        controller.reference_limit = 4;
        controller.ema_elapsed_ms = Some(120.0);
        controller.baseline_elapsed_ms = Some(120.0);
        controller.previous_efficiency = Some(4.0 / 120.0);

        controller.current_limit = 6;
        controller.record_success(Duration::from_millis(2_400), 12);

        assert_eq!(controller.current_limit, 6);
        assert_eq!(controller.reference_limit, 6);
        assert_eq!(controller.baseline_elapsed_ms, Some(2_400.0));
        assert_eq!(
            controller.last_adjustment,
            AdaptiveConcurrencyAdjustment::Stable
        );
    }

    #[test]
    fn debug_snapshot_requires_sustained_io_pressure_before_contracting() {
        let mut controller = AdaptiveConcurrencyController::new_for_test(8);
        controller.current_limit = 6;
        controller.reference_limit = 6;
        controller.ema_elapsed_ms = Some(100.0);
        controller.baseline_elapsed_ms = Some(100.0);
        controller.previous_efficiency = Some(6.0 / 100.0);

        controller.record_success(Duration::from_millis(450), 12);

        assert_eq!(controller.current_limit, 6);
        assert_eq!(
            controller.last_adjustment,
            AdaptiveConcurrencyAdjustment::ObservedIoPressure
        );
        assert_eq!(controller.io_pressure_streak, 1);
    }
}
