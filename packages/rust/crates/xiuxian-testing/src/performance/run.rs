use std::collections::BTreeMap;
use std::fmt::Display;
use std::future::Future;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures::future::join_all;
use hdrhistogram::Histogram;

use crate::performance::report::{PERF_REPORT_SCHEMA_VERSION, persist_report};
use crate::performance::types::{PerfQuantiles, PerfReport, PerfRunConfig, PerfSummary};

const HISTOGRAM_MAX_MICROS: u32 = 3_600_000_000;

fn now_unix_ms() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => u64::try_from(duration.as_millis()).unwrap_or(u64::MAX),
        Err(_) => 0,
    }
}

fn duration_to_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn sample_ms_to_histogram_micros(sample_ms: f64) -> u32 {
    if !sample_ms.is_finite() || sample_ms <= 0.0 {
        return 1;
    }

    let micros = Duration::from_secs_f64(sample_ms / 1_000.0).as_micros();
    let clipped = micros.clamp(1, u128::from(HISTOGRAM_MAX_MICROS));
    u32::try_from(clipped).unwrap_or(HISTOGRAM_MAX_MICROS)
}

fn bounded_u64_to_f64(value: u64) -> f64 {
    f64::from(u32::try_from(value).unwrap_or(u32::MAX))
}

fn bounded_usize_to_f64(value: usize) -> f64 {
    f64::from(u32::try_from(value).unwrap_or(u32::MAX))
}

fn percentile_index(len: usize, numerator: usize, denominator: usize) -> usize {
    if len <= 1 {
        return 0;
    }

    let max_index = len.saturating_sub(1);
    let rounded = max_index
        .saturating_mul(numerator)
        .saturating_add(denominator / 2)
        / denominator;
    rounded.min(max_index)
}

fn summarize_quantiles(samples_ms: &[f64]) -> PerfQuantiles {
    if samples_ms.is_empty() {
        return PerfQuantiles::default();
    }

    let Ok(mut histogram) =
        Histogram::<u32>::new_with_bounds(1, u64::from(HISTOGRAM_MAX_MICROS), 3)
    else {
        return summarize_quantiles_fallback(samples_ms);
    };

    for sample in samples_ms {
        let value = sample_ms_to_histogram_micros(*sample);
        let _ = histogram.record(u64::from(value));
    }

    if histogram.is_empty() {
        return summarize_quantiles_fallback(samples_ms);
    }

    PerfQuantiles {
        min_ms: bounded_u64_to_f64(histogram.min()) / 1_000.0,
        mean_ms: histogram.mean() / 1_000.0,
        max_ms: bounded_u64_to_f64(histogram.max()) / 1_000.0,
        p50_ms: bounded_u64_to_f64(histogram.value_at_quantile(0.50)) / 1_000.0,
        p95_ms: bounded_u64_to_f64(histogram.value_at_quantile(0.95)) / 1_000.0,
        p99_ms: bounded_u64_to_f64(histogram.value_at_quantile(0.99)) / 1_000.0,
    }
}

fn summarize_quantiles_fallback(samples_ms: &[f64]) -> PerfQuantiles {
    if samples_ms.is_empty() {
        return PerfQuantiles::default();
    }

    let mut sorted = samples_ms.to_vec();
    sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let len = sorted.len();
    let sum: f64 = sorted.iter().sum();
    PerfQuantiles {
        min_ms: sorted[0],
        mean_ms: sum / bounded_usize_to_f64(len),
        max_ms: sorted[len - 1],
        p50_ms: sorted[percentile_index(len, 50, 100)],
        p95_ms: sorted[percentile_index(len, 95, 100)],
        p99_ms: sorted[percentile_index(len, 99, 100)],
    }
}

fn build_metadata(mode: &str) -> BTreeMap<String, String> {
    let mut metadata = BTreeMap::new();
    metadata.insert("mode".to_string(), mode.to_string());

    if let Ok(value) = std::env::var("CARGO_PKG_NAME") {
        metadata.insert("crate".to_string(), value);
    }
    if let Ok(value) = std::env::var("PRJ_ROOT") {
        metadata.insert("project_root".to_string(), value);
    }
    if let Ok(value) = std::env::var("PRJ_RUNTIME_DIR") {
        metadata.insert("runtime_dir".to_string(), value);
    }

    metadata
}

fn build_summary(
    total_ops: u64,
    success_ops: u64,
    timeout_ops: u64,
    error_ops: u64,
    elapsed: Duration,
) -> PerfSummary {
    let failed_ops = timeout_ops.saturating_add(error_ops);
    let elapsed_secs = elapsed.as_secs_f64();
    let throughput_qps = if elapsed_secs > 0.0 {
        bounded_u64_to_f64(success_ops) / elapsed_secs
    } else {
        0.0
    };
    let error_rate = if total_ops > 0 {
        bounded_u64_to_f64(failed_ops) / bounded_u64_to_f64(total_ops)
    } else {
        0.0
    };

    PerfSummary {
        total_ops,
        success_ops,
        timeout_ops,
        error_ops,
        error_rate,
        throughput_qps,
        elapsed_ms: duration_to_ms(elapsed),
    }
}

struct RunMetrics {
    total_ops: u64,
    success_ops: u64,
    timeout_ops: u64,
    error_ops: u64,
    samples_ms: Vec<f64>,
}

fn finalize_report(
    suite: &str,
    case: &str,
    mode: &str,
    config: PerfRunConfig,
    elapsed: Duration,
    metrics: RunMetrics,
) -> PerfReport {
    let quantiles = summarize_quantiles(&metrics.samples_ms);
    let summary = build_summary(
        metrics.total_ops,
        metrics.success_ops,
        metrics.timeout_ops,
        metrics.error_ops,
        elapsed,
    );
    let captured_at_unix_ms = now_unix_ms();
    let mut report = PerfReport {
        schema_version: PERF_REPORT_SCHEMA_VERSION.to_string(),
        suite: suite.to_string(),
        case: case.to_string(),
        mode: mode.to_string(),
        captured_at_unix_ms,
        run_config: config,
        summary,
        quantiles,
        sample_latency_ms: metrics.samples_ms,
        metadata: build_metadata(mode),
        report_path: None,
    };

    match persist_report(&mut report) {
        Ok(_) => {}
        Err(error) => report.add_metadata("report_write_error", error.to_string()),
    }

    report
}

/// Run sync operation sampling and return a persisted performance report.
#[must_use]
pub fn run_sync_budget<T, E, F>(
    suite: &str,
    case: &str,
    config: &PerfRunConfig,
    mut operation: F,
) -> PerfReport
where
    F: FnMut() -> Result<T, E>,
    E: Display,
{
    let config = config.normalized();
    let timeout = config.timeout();

    for _ in 0..config.warmup_samples {
        for _ in 0..config.concurrency {
            let _ = operation();
        }
    }

    let started = Instant::now();
    let mut total_ops = 0_u64;
    let mut success_ops = 0_u64;
    let mut timeout_ops = 0_u64;
    let mut error_ops = 0_u64;
    let mut samples_ms = Vec::with_capacity(config.samples * config.concurrency);

    for _ in 0..config.samples {
        for _ in 0..config.concurrency {
            total_ops = total_ops.saturating_add(1);
            let op_started = Instant::now();
            let result = operation();
            let elapsed = op_started.elapsed();
            samples_ms.push(duration_to_ms(elapsed));

            if elapsed > timeout {
                timeout_ops = timeout_ops.saturating_add(1);
                continue;
            }

            match result {
                Ok(_) => {
                    success_ops = success_ops.saturating_add(1);
                }
                Err(_) => {
                    error_ops = error_ops.saturating_add(1);
                }
            }
        }
    }

    finalize_report(
        suite,
        case,
        "sync",
        config,
        started.elapsed(),
        RunMetrics {
            total_ops,
            success_ops,
            timeout_ops,
            error_ops,
            samples_ms,
        },
    )
}

struct AsyncOutcome {
    elapsed: Duration,
    timed_out: bool,
    failed: bool,
}

async fn run_one_async<T, E, Fut>(future: Fut, timeout: Duration) -> AsyncOutcome
where
    Fut: Future<Output = Result<T, E>>,
{
    let started = Instant::now();
    match tokio::time::timeout(timeout, future).await {
        Ok(Ok(_)) => AsyncOutcome {
            elapsed: started.elapsed(),
            timed_out: false,
            failed: false,
        },
        Ok(Err(_)) => AsyncOutcome {
            elapsed: started.elapsed(),
            timed_out: false,
            failed: true,
        },
        Err(_) => AsyncOutcome {
            elapsed: started.elapsed(),
            timed_out: true,
            failed: false,
        },
    }
}

/// Run async operation sampling and return a persisted performance report.
#[must_use]
pub async fn run_async_budget<T, E, Fut, F>(
    suite: &str,
    case: &str,
    config: &PerfRunConfig,
    operation: F,
) -> PerfReport
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let config = config.normalized();
    let timeout = config.timeout();

    for _ in 0..config.warmup_samples {
        for _ in 0..config.concurrency {
            let _ = tokio::time::timeout(timeout, operation()).await;
        }
    }

    let started = Instant::now();
    let mut total_ops = 0_u64;
    let mut success_ops = 0_u64;
    let mut timeout_ops = 0_u64;
    let mut error_ops = 0_u64;
    let mut samples_ms = Vec::with_capacity(config.samples * config.concurrency);

    for _ in 0..config.samples {
        let mut batch = Vec::with_capacity(config.concurrency);
        for _ in 0..config.concurrency {
            batch.push(run_one_async(operation(), timeout));
        }

        for outcome in join_all(batch).await {
            total_ops = total_ops.saturating_add(1);
            samples_ms.push(duration_to_ms(outcome.elapsed));
            if outcome.timed_out {
                timeout_ops = timeout_ops.saturating_add(1);
            } else if outcome.failed {
                error_ops = error_ops.saturating_add(1);
            } else {
                success_ops = success_ops.saturating_add(1);
            }
        }
    }

    finalize_report(
        suite,
        case,
        "async",
        config,
        started.elapsed(),
        RunMetrics {
            total_ops,
            success_ops,
            timeout_ops,
            error_ops,
            samples_ms,
        },
    )
}
