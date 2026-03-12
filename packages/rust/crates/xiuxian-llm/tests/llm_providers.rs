use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use xiuxian_llm::llm::vision::DeepseekRuntime;

use crate::config::RuntimeSettings;
use crate::embedding::EmbeddingClient;

use super::embedding::{
    SharedEmbeddingRuntime, resolve_forced_http_model_host_upstream_base_url_for_tests,
    resolve_shared_embedding_backend_override,
};
use super::ocr::{
    OcrBlockingExecution, OcrHostAdmissionDecision, OcrProcessGuardDecision, OcrRequestAdmission,
    admit_deepseek_ocr_request_or_log, compute_deepseek_ocr_failure_circuit_open_until_for_tests,
    deepseek_ocr_global_lock_path_with_inputs, deepseek_ocr_memory_guard_triggered,
    deepseek_startup_prewarm_enabled_with_input, note_deepseek_ocr_cross_process_wait_acquired,
    note_deepseek_ocr_cross_process_wait_timed_out, prewarm_ocr_runtime,
    resolve_deepseek_ocr_host_admission, resolve_deepseek_ocr_max_dimension_with_inputs,
    resolve_deepseek_ocr_max_in_flight_with_inputs, resolve_deepseek_ocr_memory_limit_bytes,
    resolve_deepseek_ocr_process_guard_decision,
    resolve_deepseek_ocr_stuck_recovery_enabled_with_input,
    resolve_deepseek_ocr_stuck_recovery_exit_code_with_input,
    resolve_deepseek_ocr_timeout_with_inputs,
    resolve_gateway_ocr_max_concurrent_requests_with_inputs,
    simulate_ocr_gate_timeout_interrupt_recovery_for_tests,
    simulate_ocr_runtime_gate_bypass_for_tests, snapshot_deepseek_ocr_worker_telemetry,
    wait_for_deepseek_ocr_watchdog_deadline_for_tests,
};
use super::summary::build_model_host_summary;

fn make_embedding_runtime(host_mode: &'static str, pid: Option<u32>) -> SharedEmbeddingRuntime {
    SharedEmbeddingRuntime {
        client: Arc::new(EmbeddingClient::new("http://127.0.0.1:11434", 3)),
        default_model: Some("Qwen/Qwen3-Embedding-0.6B".to_string()),
        fallback_embedding_dim: 1024,
        backend: "mistral_sdk".to_string(),
        base_url: "inproc://mistral-sdk".to_string(),
        host_mode,
        hosted_process_pid: pid,
    }
}

#[test]
fn gateway_model_host_summary_reports_embedding_and_configured_ocr() {
    let embedding_runtime = make_embedding_runtime("managed_mistral_server", Some(4242));
    let ocr_runtime = DeepseekRuntime::Configured {
        model_root: Arc::from(".data/models/dots-ocr"),
    };

    let summary = build_model_host_summary(&embedding_runtime, &ocr_runtime);

    assert_eq!(summary.services.len(), 3);
    assert_eq!(summary.hosted_process_pids(), vec![4242]);

    let embedding = &summary.services[0];
    assert_eq!(embedding.service, "embedding");
    assert_eq!(embedding.backend, "mistral_sdk");
    assert_eq!(embedding.host_mode, "managed_mistral_server");
    assert_eq!(embedding.endpoint, "inproc://mistral-sdk");
    assert_eq!(embedding.hosted_process_pid, Some(4242));
    assert_eq!(
        embedding.detail.as_deref(),
        Some("Qwen/Qwen3-Embedding-0.6B")
    );

    let llm_host = &summary.services[1];
    assert_eq!(llm_host.service, "llm_host");
    assert_eq!(llm_host.backend, "mistralrs");
    assert_eq!(llm_host.host_mode, "managed_mistral_server");
    assert_eq!(llm_host.endpoint, "inproc://mistral-sdk");
    assert_eq!(llm_host.hosted_process_pid, Some(4242));
    assert_eq!(llm_host.detail.as_deref(), Some("openai_compatible_server"));

    let ocr = &summary.services[2];
    assert_eq!(ocr.service, "ocr");
    assert_eq!(ocr.backend, "deepseek_ocr");
    assert_eq!(ocr.host_mode, "local_native_runtime");
    assert_eq!(ocr.endpoint, "inproc://deepseek-ocr");
    assert_eq!(ocr.detail.as_deref(), Some(".data/models/dots-ocr"));
}

#[test]
fn gateway_model_host_summary_reports_remote_http_ocr() {
    let embedding_runtime = make_embedding_runtime("inproc_mistral_sdk", None);
    let ocr_runtime = DeepseekRuntime::RemoteHttp {
        base_url: Arc::from("http://127.0.0.1:18193"),
    };

    let summary = build_model_host_summary(&embedding_runtime, &ocr_runtime);

    assert_eq!(summary.services.len(), 2);

    let ocr = &summary.services[1];
    assert_eq!(ocr.service, "ocr");
    assert_eq!(ocr.backend, "deepseek_ocr");
    assert_eq!(ocr.host_mode, "remote_http_runtime");
    assert_eq!(ocr.endpoint, "http://127.0.0.1:18193");
    assert_eq!(ocr.detail, None);
}

#[test]
fn gateway_model_host_summary_reports_disabled_ocr_reason() {
    let embedding_runtime = make_embedding_runtime("inproc_mistral_sdk", None);
    let ocr_runtime = DeepseekRuntime::Disabled {
        reason: Arc::from("shared OCR client override disabled in gateway"),
    };

    let summary = build_model_host_summary(&embedding_runtime, &ocr_runtime);

    assert_eq!(summary.services.len(), 2);
    assert!(summary.hosted_process_pids().is_empty());

    let embedding = &summary.services[0];
    assert_eq!(embedding.service, "embedding");
    assert_eq!(embedding.host_mode, "inproc_mistral_sdk");

    let ocr = &summary.services[1];
    assert_eq!(ocr.host_mode, "disabled_runtime");
    assert_eq!(ocr.endpoint, "disabled://deepseek-ocr");
    assert_eq!(
        ocr.detail.as_deref(),
        Some("shared OCR client override disabled in gateway")
    );
}

#[test]
fn shared_embedding_backend_override_prefers_env_backend() {
    let mut settings = RuntimeSettings::default();
    settings.memory.embedding_backend = Some("mistral_sdk".to_string());

    let resolved =
        resolve_shared_embedding_backend_override(&settings, Some("openai_http"), None, false);

    assert_eq!(resolved.as_deref(), Some("openai_http"));
}

#[test]
fn shared_embedding_backend_override_forces_http_for_mistral_sdk() {
    let mut settings = RuntimeSettings::default();
    settings.embedding.backend = Some("mistral_sdk".to_string());

    let resolved = resolve_shared_embedding_backend_override(&settings, None, None, false);

    assert_eq!(resolved.as_deref(), Some("http"));
}

#[test]
fn forced_http_model_host_upstream_ignores_memory_gateway_base_url() {
    let mut settings = RuntimeSettings::default();
    settings.memory.embedding_backend = Some("mistral_sdk".to_string());
    settings.memory.embedding_base_url = Some("http://127.0.0.1:18092".to_string());

    let resolved =
        resolve_forced_http_model_host_upstream_base_url_for_tests(&settings, None, None);

    assert_eq!(resolved, "http://localhost:11434");
}

#[test]
fn forced_http_model_host_upstream_ignores_embed_base_url_when_it_matches_gateway() {
    let mut settings = RuntimeSettings::default();
    settings.memory.embedding_backend = Some("mistral_sdk".to_string());
    settings.memory.embedding_base_url = Some("http://127.0.0.1:18092".to_string());

    let resolved = resolve_forced_http_model_host_upstream_base_url_for_tests(
        &settings,
        Some("http://127.0.0.1:18092"),
        Some("http://127.0.0.1:18092"),
    );

    assert_eq!(resolved, "http://localhost:11434");
}

#[test]
fn forced_http_model_host_upstream_prefers_explicit_embed_upstream() {
    let mut settings = RuntimeSettings::default();
    settings.memory.embedding_backend = Some("mistral_sdk".to_string());
    settings.memory.embedding_base_url = Some("http://127.0.0.1:18092".to_string());
    settings.embedding.client_url = Some("http://127.0.0.1:3002".to_string());

    let resolved = resolve_forced_http_model_host_upstream_base_url_for_tests(
        &settings,
        None,
        Some("http://127.0.0.1:11434"),
    );

    assert_eq!(resolved, "http://127.0.0.1:11434");
}

#[test]
fn deepseek_startup_prewarm_defaults_to_enabled() {
    assert!(deepseek_startup_prewarm_enabled_with_input(None));
    assert!(deepseek_startup_prewarm_enabled_with_input(Some("1")));
    assert!(deepseek_startup_prewarm_enabled_with_input(Some("true")));
}

#[test]
fn deepseek_startup_prewarm_honors_false_like_values() {
    assert!(!deepseek_startup_prewarm_enabled_with_input(Some("0")));
    assert!(!deepseek_startup_prewarm_enabled_with_input(Some("false")));
    assert!(!deepseek_startup_prewarm_enabled_with_input(Some(" no ")));
    assert!(!deepseek_startup_prewarm_enabled_with_input(Some("Off")));
}

#[tokio::test]
async fn remote_http_ocr_prewarm_stays_safe_inside_async_runtime() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut request = [0_u8; 2048];
        let bytes_read = stream.read(&mut request).await.unwrap();
        let request_text = String::from_utf8_lossy(&request[..bytes_read]);
        assert!(request_text.starts_with("POST /v1/vision/ocr/prewarm "));
        let body = r#"{"ready":true}"#;
        let response = format!(
            "HTTP/1.1 200 OK
content-type: application/json
content-length: {}
connection: close

{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).await.unwrap();
    });

    let runtime = Arc::new(DeepseekRuntime::RemoteHttp {
        base_url: Arc::from(format!("http://{address}")),
    });

    prewarm_ocr_runtime(runtime).await.unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn remote_http_ocr_prewarm_is_best_effort_when_upstream_is_unavailable() {
    let runtime = Arc::new(DeepseekRuntime::RemoteHttp {
        base_url: Arc::from("http://127.0.0.1:9"),
    });

    prewarm_ocr_runtime(runtime).await.unwrap();
}

#[test]
fn deepseek_ocr_max_in_flight_prefers_env_then_config_then_default() {
    assert_eq!(
        resolve_deepseek_ocr_max_in_flight_with_inputs(Some("8"), Some(4)),
        8
    );
    assert_eq!(
        resolve_deepseek_ocr_max_in_flight_with_inputs(None, Some(4)),
        4
    );
    assert_eq!(
        resolve_deepseek_ocr_max_in_flight_with_inputs(Some("0"), Some(4)),
        4
    );
    assert_eq!(
        resolve_deepseek_ocr_max_in_flight_with_inputs(None, None),
        1
    );
}

#[test]
fn deepseek_ocr_max_dimension_prefers_env_then_config_then_default() {
    assert_eq!(
        resolve_deepseek_ocr_max_dimension_with_inputs(Some("640"), Some(768)),
        640
    );
    assert_eq!(
        resolve_deepseek_ocr_max_dimension_with_inputs(None, Some(768)),
        768
    );
    assert_eq!(
        resolve_deepseek_ocr_max_dimension_with_inputs(Some("0"), Some(768)),
        768
    );
    assert_eq!(
        resolve_deepseek_ocr_max_dimension_with_inputs(None, None),
        1024
    );
}

#[test]
fn gateway_ocr_max_concurrent_requests_prefers_explicit_gateway_then_worker_limit() {
    assert_eq!(
        resolve_gateway_ocr_max_concurrent_requests_with_inputs(Some("9"), Some(6), Some(4)),
        9
    );
    assert_eq!(
        resolve_gateway_ocr_max_concurrent_requests_with_inputs(None, Some(6), Some(4)),
        6
    );
    assert_eq!(
        resolve_gateway_ocr_max_concurrent_requests_with_inputs(None, None, Some(4)),
        4
    );
    assert_eq!(
        resolve_gateway_ocr_max_concurrent_requests_with_inputs(Some("0"), None, None),
        1
    );
}

#[test]
fn deepseek_ocr_global_lock_path_defaults_to_project_runtime_dir() {
    let path = deepseek_ocr_global_lock_path_with_inputs(Path::new("/tmp/project"), None, None);
    assert_eq!(
        path,
        PathBuf::from("/tmp/project/.run/locks/deepseek-ocr.lock")
    );
}

#[test]
fn deepseek_ocr_global_lock_path_prefers_custom_override() {
    let path = deepseek_ocr_global_lock_path_with_inputs(
        Path::new("/tmp/project"),
        Some(Path::new(".runtime")),
        Some(Path::new("locks/custom-ocr.lock")),
    );
    assert_eq!(path, PathBuf::from("/tmp/project/locks/custom-ocr.lock"));
}

#[test]
fn deepseek_ocr_failure_circuit_open_until_uses_latest_deadline() {
    let open_until = compute_deepseek_ocr_failure_circuit_open_until_for_tests(2_000, 1_000, 250)
        .expect("cooldown should open circuit");
    assert_eq!(open_until, 2_000);

    let later_open_until =
        compute_deepseek_ocr_failure_circuit_open_until_for_tests(500, 1_000, 250)
            .expect("cooldown should open circuit");
    assert_eq!(later_open_until, 1_250);
}

#[test]
fn deepseek_ocr_worker_telemetry_tracks_cross_process_wait_counters() {
    let before = snapshot_deepseek_ocr_worker_telemetry(0);

    note_deepseek_ocr_cross_process_wait_acquired();
    note_deepseek_ocr_cross_process_wait_timed_out();

    let after = snapshot_deepseek_ocr_worker_telemetry(0);
    assert_eq!(
        after.total_cross_process_wait_acquired,
        before.total_cross_process_wait_acquired.saturating_add(1)
    );
    assert_eq!(
        after.total_cross_process_wait_timed_out,
        before.total_cross_process_wait_timed_out.saturating_add(1)
    );
}

#[test]
fn deepseek_ocr_watchdog_wait_returns_early_when_worker_finishes() {
    let worker_done = Arc::new(AtomicBool::new(false));
    let done_for_thread = Arc::clone(&worker_done);
    let setter = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(20));
        done_for_thread.store(true, Ordering::Relaxed);
    });

    let started = Instant::now();
    let reached_deadline = wait_for_deepseek_ocr_watchdog_deadline_for_tests(
        worker_done.as_ref(),
        Duration::from_millis(500),
        Duration::from_millis(5),
    );
    setter.join().expect("setter thread should finish cleanly");

    assert!(!reached_deadline);
    assert!(started.elapsed() < Duration::from_millis(200));
}

#[test]
fn deepseek_ocr_watchdog_wait_reaches_deadline_when_worker_stays_busy() {
    let worker_done = AtomicBool::new(false);
    let started = Instant::now();

    let reached_deadline = wait_for_deepseek_ocr_watchdog_deadline_for_tests(
        &worker_done,
        Duration::from_millis(30),
        Duration::from_millis(5),
    );

    assert!(reached_deadline);
    assert!(started.elapsed() >= Duration::from_millis(25));
    assert!(started.elapsed() < Duration::from_millis(200));
}

#[test]
fn deepseek_ocr_stuck_recovery_exit_flag_recognizes_truthy_values() {
    assert!(resolve_deepseek_ocr_stuck_recovery_enabled_with_input(
        Some("true")
    ));
    assert!(resolve_deepseek_ocr_stuck_recovery_enabled_with_input(
        Some("1")
    ));
    assert!(!resolve_deepseek_ocr_stuck_recovery_enabled_with_input(
        Some("false")
    ));
    assert!(!resolve_deepseek_ocr_stuck_recovery_enabled_with_input(
        None
    ));
}

#[test]
fn deepseek_ocr_stuck_recovery_exit_code_uses_positive_override_or_default() {
    assert_eq!(
        resolve_deepseek_ocr_stuck_recovery_exit_code_with_input(Some("91")),
        91
    );
    assert_eq!(
        resolve_deepseek_ocr_stuck_recovery_exit_code_with_input(Some("0")),
        75
    );
    assert_eq!(
        resolve_deepseek_ocr_stuck_recovery_exit_code_with_input(None),
        75
    );
}

#[test]
fn deepseek_ocr_timeout_uses_cold_stage_and_clamps_cold_budget_to_warm_floor() {
    let timeout = resolve_deepseek_ocr_timeout_with_inputs(true, Some(60_000), Some(1_000));
    assert_eq!(timeout.stage, "cold_start");
    assert_eq!(timeout.duration, std::time::Duration::from_millis(60_000));
}

#[test]
fn deepseek_ocr_timeout_uses_warm_budget_in_steady_state() {
    let timeout = resolve_deepseek_ocr_timeout_with_inputs(false, Some(45_000), Some(90_000));
    assert_eq!(timeout.stage, "steady_state");
    assert_eq!(timeout.duration, std::time::Duration::from_millis(45_000));
}

#[test]
fn deepseek_ocr_memory_limit_bytes_parse_fractional_gib() {
    let limit_bytes =
        resolve_deepseek_ocr_memory_limit_bytes(Some("1.5")).expect("fractional GiB should parse");
    assert_eq!(limit_bytes, 1_610_612_736);
}

#[test]
fn deepseek_ocr_memory_limit_bytes_reject_invalid_values() {
    assert_eq!(resolve_deepseek_ocr_memory_limit_bytes(Some("0")), None);
    assert_eq!(resolve_deepseek_ocr_memory_limit_bytes(Some("-1")), None);
    assert_eq!(resolve_deepseek_ocr_memory_limit_bytes(Some("abc")), None);
}

#[test]
fn deepseek_ocr_memory_guard_triggered_when_rss_exceeds_limit() {
    assert!(deepseek_ocr_memory_guard_triggered(
        Some("1.0"),
        1_073_741_825
    ));
    assert!(!deepseek_ocr_memory_guard_triggered(
        Some("1.0"),
        1_073_741_824
    ));
}

#[tokio::test]
async fn deepseek_ocr_process_guard_decision_skips_remote_runtime() {
    let runtime = DeepseekRuntime::RemoteHttp {
        base_url: Arc::from("http://127.0.0.1:9999"),
    };

    let decision = resolve_deepseek_ocr_process_guard_decision(&runtime).await;

    assert!(matches!(decision, OcrProcessGuardDecision::NotRequired));
}

#[tokio::test]
async fn deepseek_ocr_host_admission_allows_remote_runtime_without_guard() {
    let runtime = DeepseekRuntime::RemoteHttp {
        base_url: Arc::from("http://127.0.0.1:9999"),
    };

    let decision = resolve_deepseek_ocr_host_admission(&runtime).await;

    assert!(matches!(
        decision,
        OcrHostAdmissionDecision::Allowed { guard: None }
    ));
}

#[tokio::test]
async fn deepseek_ocr_request_admission_keeps_remote_runtime_allowed() {
    let runtime = DeepseekRuntime::RemoteHttp {
        base_url: Arc::from("http://127.0.0.1:9999"),
    };

    let admission = admit_deepseek_ocr_request_or_log(&runtime).await;

    assert!(matches!(
        admission,
        OcrRequestAdmission::Allowed { guard: None }
    ));
}
#[tokio::test]
async fn remote_ocr_runtime_bypasses_saturated_local_gate() {
    let outcome = simulate_ocr_runtime_gate_bypass_for_tests(true).await;

    assert!(matches!(outcome, OcrBlockingExecution::Completed(())));
}

#[tokio::test]
async fn local_ocr_runtime_respects_saturated_local_gate() {
    let outcome = simulate_ocr_runtime_gate_bypass_for_tests(false).await;

    assert!(matches!(
        outcome,
        OcrBlockingExecution::Busy | OcrBlockingExecution::BusyBackpressure
    ));
}

#[tokio::test]
async fn timed_out_ocr_worker_sets_interrupt_signal_and_recovers_gate() {
    let probe = simulate_ocr_gate_timeout_interrupt_recovery_for_tests(25).await;

    assert_eq!(
        probe.first_outcome,
        super::ocr::OcrProbeFirstOutcome::TimedOut
    );
    assert!(probe.second_was_busy || probe.second_completed);
    assert!(probe.recovered_after_wait);
}
