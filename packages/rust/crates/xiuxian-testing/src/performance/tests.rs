use std::panic::catch_unwind;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use super::{
    PERF_REPORT_SCHEMA_VERSION, PerfBudget, PerfRunConfig, assert_perf_budget, run_async_budget,
    run_sync_budget,
};

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(msg) = payload.downcast_ref::<String>() {
        return msg.clone();
    }
    if let Some(msg) = payload.downcast_ref::<&str>() {
        return (*msg).to_string();
    }
    "unknown panic payload".to_string()
}

#[test]
fn run_sync_budget_collects_quantiles_and_persists_report() {
    let config = PerfRunConfig {
        warmup_samples: 1,
        samples: 8,
        timeout_ms: 100,
        concurrency: 1,
    };
    let counter = Arc::new(AtomicUsize::new(0));
    let report = run_sync_budget(
        "xiuxian-testing/perf",
        "sync_quantiles",
        &config,
        || -> Result<(), &'static str> {
            let loops = counter.fetch_add(1, Ordering::Relaxed) % 2_000 + 1_000;
            for _ in 0..loops {
                std::hint::spin_loop();
            }
            Ok(())
        },
    );

    assert_eq!(report.schema_version, PERF_REPORT_SCHEMA_VERSION);
    assert_eq!(report.summary.total_ops, 8);
    assert_eq!(report.summary.success_ops, 8);
    assert!(report.quantiles.p95_ms >= report.quantiles.p50_ms);
    assert!(report.report_path.is_some());
}

#[tokio::test(flavor = "current_thread")]
async fn run_async_budget_tracks_timeout_and_errors() {
    let config = PerfRunConfig {
        warmup_samples: 0,
        samples: 4,
        timeout_ms: 5,
        concurrency: 2,
    };
    let counter = Arc::new(AtomicUsize::new(0));
    let report = run_async_budget(
        "xiuxian-testing/perf",
        "async_timeout_error",
        &config,
        || {
            let counter = Arc::clone(&counter);
            async move {
                let turn = counter.fetch_add(1, Ordering::Relaxed);
                if turn.is_multiple_of(3) {
                    tokio::time::sleep(Duration::from_millis(15)).await;
                    Ok::<(), &'static str>(())
                } else if turn.is_multiple_of(2) {
                    Err::<(), &'static str>("synthetic-error")
                } else {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    Ok::<(), &'static str>(())
                }
            }
        },
    )
    .await;

    assert_eq!(report.summary.total_ops, 8);
    assert!(report.summary.timeout_ops > 0);
    assert!(report.summary.error_ops > 0);
}

#[test]
fn assert_perf_budget_emits_stable_failure_message() {
    let config = PerfRunConfig {
        warmup_samples: 0,
        samples: 3,
        timeout_ms: 1_000,
        concurrency: 1,
    };
    let report = run_sync_budget("xiuxian-testing/perf", "budget_message", &config, || {
        std::thread::sleep(Duration::from_millis(2));
        Ok::<(), &'static str>(())
    });

    let budget = PerfBudget {
        max_p95_latency_ms: Some(0.1),
        min_throughput_qps: Some(100_000.0),
        ..PerfBudget::new()
    };

    let panic_result = catch_unwind(|| assert_perf_budget(&report, &budget));
    let payload = match panic_result {
        Ok(()) => panic!("expected budget assertion to fail"),
        Err(payload) => payload,
    };
    let message = panic_message(payload);
    assert!(message.contains("performance budget gate failed"));
    assert!(message.contains("p95 latency exceeded"));
    assert!(message.contains("throughput below floor"));
}

#[test]
fn persisted_report_contains_required_schema_fields() {
    let config = PerfRunConfig {
        warmup_samples: 0,
        samples: 2,
        timeout_ms: 20,
        concurrency: 1,
    };
    let report = run_sync_budget("xiuxian-testing/perf", "schema_fields", &config, || {
        Ok::<(), &'static str>(())
    });

    let Some(path) = report.report_path.clone() else {
        panic!("expected persisted report path");
    };

    let payload = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("expected report file at {path}: {error}"));
    let value: serde_json::Value = serde_json::from_str(&payload)
        .unwrap_or_else(|error| panic!("expected valid json payload at {path}: {error}"));

    assert_eq!(
        value
            .get("schema_version")
            .and_then(serde_json::Value::as_str),
        Some(PERF_REPORT_SCHEMA_VERSION)
    );
    assert!(value.get("summary").is_some());
    assert!(value.get("quantiles").is_some());
    assert!(value.get("sample_latency_ms").is_some());
    assert!(value.get("metadata").is_some());
}
