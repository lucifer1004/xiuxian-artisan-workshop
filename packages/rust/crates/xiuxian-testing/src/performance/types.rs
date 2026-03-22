use std::collections::BTreeMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Performance budget thresholds used by `assert_perf_budget`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerfBudget {
    /// Maximum allowed p50 latency in milliseconds.
    pub max_p50_latency_ms: Option<f64>,
    /// Maximum allowed p95 latency in milliseconds.
    pub max_p95_latency_ms: Option<f64>,
    /// Maximum allowed p99 latency in milliseconds.
    pub max_p99_latency_ms: Option<f64>,
    /// Minimum required throughput in operations/second.
    pub min_throughput_qps: Option<f64>,
    /// Maximum allowed failure rate (`0.0..=1.0`).
    pub max_error_rate: Option<f64>,
}

impl PerfBudget {
    /// Create an empty budget that can be filled incrementally.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            max_p50_latency_ms: None,
            max_p95_latency_ms: None,
            max_p99_latency_ms: None,
            min_throughput_qps: None,
            max_error_rate: None,
        }
    }
}

/// Runtime configuration for performance sampling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfRunConfig {
    /// Number of warmup rounds run before measured samples.
    pub warmup_samples: usize,
    /// Number of measured rounds.
    pub samples: usize,
    /// Per-operation timeout in milliseconds.
    pub timeout_ms: u64,
    /// Worker count per round.
    pub concurrency: usize,
}

impl Default for PerfRunConfig {
    fn default() -> Self {
        Self {
            warmup_samples: 3,
            samples: 24,
            timeout_ms: 1_000,
            concurrency: 1,
        }
    }
}

impl PerfRunConfig {
    /// Build a normalized config with minimum safe values.
    #[must_use]
    pub fn normalized(&self) -> Self {
        Self {
            warmup_samples: self.warmup_samples,
            samples: self.samples.max(1),
            timeout_ms: self.timeout_ms.max(1),
            concurrency: self.concurrency.max(1),
        }
    }

    /// Timeout as [`Duration`].
    #[must_use]
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_ms.max(1))
    }
}

/// Aggregated latency quantiles in milliseconds.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[allow(clippy::struct_field_names)]
pub struct PerfQuantiles {
    /// Minimum measured latency.
    pub min_ms: f64,
    /// Mean measured latency.
    pub mean_ms: f64,
    /// Maximum measured latency.
    pub max_ms: f64,
    /// p50 latency.
    pub p50_ms: f64,
    /// p95 latency.
    pub p95_ms: f64,
    /// p99 latency.
    pub p99_ms: f64,
}

/// Summary counters and top-line KPIs.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerfSummary {
    /// Total operations attempted.
    pub total_ops: u64,
    /// Successful operations.
    pub success_ops: u64,
    /// Timed out operations.
    pub timeout_ops: u64,
    /// Error operations (excluding timeout category).
    pub error_ops: u64,
    /// Failure rate as `(timeout_ops + error_ops) / total_ops`.
    pub error_rate: f64,
    /// Throughput measured as successful operations per second.
    pub throughput_qps: f64,
    /// Total wall-clock duration of measured rounds.
    pub elapsed_ms: f64,
}

/// JSON-serializable performance run report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfReport {
    /// Schema identifier for report compatibility.
    pub schema_version: String,
    /// Suite namespace used for report path grouping.
    pub suite: String,
    /// Case name inside the suite.
    pub case: String,
    /// Runner mode (`sync` or `async`).
    pub mode: String,
    /// Capture timestamp in unix milliseconds.
    pub captured_at_unix_ms: u64,
    /// Run-time sampling configuration.
    pub run_config: PerfRunConfig,
    /// Top-line summary metrics.
    pub summary: PerfSummary,
    /// Latency quantiles from measured samples.
    pub quantiles: PerfQuantiles,
    /// Raw measured sample latencies in milliseconds.
    pub sample_latency_ms: Vec<f64>,
    /// Free-form metadata for CI, host info, and other dimensions.
    pub metadata: BTreeMap<String, String>,
    /// Absolute report path when persisted.
    pub report_path: Option<String>,
}

impl PerfReport {
    /// Add one metadata key-value pair.
    pub fn add_metadata<K, V>(&mut self, key: K, value: V)
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.metadata.insert(key.into(), value.into());
    }
}
