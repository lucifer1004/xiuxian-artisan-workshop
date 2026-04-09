use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn config_default_values() {
    let config = ArtifactObserverConfig::default();
    assert!(config.enabled);
    assert_eq!(config.trace_base_path, ".cognitive/traces");
    assert!(config.ingest_on_exit);
    assert!(config.ingest_on_early_halt);
}

#[test]
fn config_clone_preserves_values() {
    let config = ArtifactObserverConfig {
        enabled: false,
        trace_base_path: "custom/path".to_string(),
        ingest_on_exit: false,
        ingest_on_early_halt: true,
    };
    let cloned = config.clone();
    assert!(!cloned.enabled);
    assert_eq!(cloned.trace_base_path, "custom/path");
    assert!(!cloned.ingest_on_exit);
    assert!(cloned.ingest_on_early_halt);
}

#[test]
fn ingestion_result_ingested() {
    let result = ArtifactIngestionResult::Ingested {
        trace_id: "trace-123".to_string(),
        anchor_id: "anchor-456".to_string(),
    };
    match result {
        ArtifactIngestionResult::Ingested {
            trace_id,
            anchor_id,
        } => {
            assert_eq!(trace_id, "trace-123");
            assert_eq!(anchor_id, "anchor-456");
        }
        _ => panic!("expected Ingested variant"),
    }
}

#[test]
fn ingestion_result_no_artifact() {
    let result = ArtifactIngestionResult::NoArtifact;
    assert!(matches!(result, ArtifactIngestionResult::NoArtifact));
}

#[test]
fn ingestion_result_skipped() {
    let result = ArtifactIngestionResult::Skipped {
        reason: Arc::from("test skip"),
    };
    match result {
        ArtifactIngestionResult::Skipped { reason } => {
            assert_eq!(reason.as_ref(), "test skip");
        }
        _ => panic!("expected Skipped variant"),
    }
}

#[test]
fn ingestion_result_failed() {
    let result = ArtifactIngestionResult::Failed {
        error: Arc::from("test error"),
    };
    match result {
        ArtifactIngestionResult::Failed { error } => {
            assert_eq!(error.as_ref(), "test error");
        }
        _ => panic!("expected Failed variant"),
    }
}

#[test]
fn ingestion_result_clone() {
    let result = ArtifactIngestionResult::Ingested {
        trace_id: "trace-789".to_string(),
        anchor_id: "anchor-012".to_string(),
    };
    let cloned = result.clone();
    assert_eq!(result, cloned);
}

#[test]
fn ingestion_result_partial_eq() {
    let result1 = ArtifactIngestionResult::Ingested {
        trace_id: "trace-1".to_string(),
        anchor_id: "anchor-1".to_string(),
    };
    let result2 = ArtifactIngestionResult::Ingested {
        trace_id: "trace-1".to_string(),
        anchor_id: "anchor-1".to_string(),
    };
    let result3 = ArtifactIngestionResult::Ingested {
        trace_id: "trace-2".to_string(),
        anchor_id: "anchor-1".to_string(),
    };
    assert_eq!(result1, result2);
    assert_ne!(result1, result3);
}

#[tokio::test]
async fn noop_sink_returns_ok() {
    let sink = NoopWendaoIngestionSink;
    let trace = CognitiveTraceRecord::new(
        "trace-test".to_string(),
        None,
        "TestNode".to_string(),
        "Test intent".to_string(),
    );
    let doc = trace.to_semantic_document("doc-1", "path.md");
    let result = sink.ingest_trace(&trace, &doc).await;
    assert_eq!(result.as_deref(), Ok("noop:trace-test"));
}

#[test]
fn observer_default_creation() {
    let observer = ArtifactObserver::default();
    assert!(observer.config().enabled);
}

#[test]
fn observer_should_handle_exit_event() {
    let observer = ArtifactObserver::default();
    let event = SwarmEvent::NodeTransition {
        session_id: Some("session-1".to_string()),
        agent_id: None,
        role_class: None,
        node_id: "TestNode".to_string(),
        phase: NodeTransitionPhase::Exiting,
        timestamp_ms: 1_700_000_000_000,
    };
    assert!(observer.should_handle_event(&event));
}

#[test]
fn observer_should_handle_failed_event() {
    let observer = ArtifactObserver::default();
    let event = SwarmEvent::NodeTransition {
        session_id: Some("session-1".to_string()),
        agent_id: None,
        role_class: None,
        node_id: "TestNode".to_string(),
        phase: NodeTransitionPhase::Failed,
        timestamp_ms: 1_700_000_000_000,
    };
    assert!(observer.should_handle_event(&event));
}

#[test]
fn observer_should_not_handle_entering_event() {
    let observer = ArtifactObserver::default();
    let event = SwarmEvent::NodeTransition {
        session_id: Some("session-1".to_string()),
        agent_id: None,
        role_class: None,
        node_id: "TestNode".to_string(),
        phase: NodeTransitionPhase::Entering,
        timestamp_ms: 1_700_000_000_000,
    };
    assert!(!observer.should_handle_event(&event));
}

#[test]
fn observer_disabled_ignores_events() {
    let config = ArtifactObserverConfig {
        enabled: false,
        ..Default::default()
    };
    let observer = ArtifactObserver::new(config, NoopWendaoIngestionSink);
    let event = SwarmEvent::NodeTransition {
        session_id: None,
        agent_id: None,
        role_class: None,
        node_id: "TestNode".to_string(),
        phase: NodeTransitionPhase::Exiting,
        timestamp_ms: 0,
    };
    assert!(!observer.should_handle_event(&event));
}

#[test]
fn observer_ignores_non_transition_events() {
    let observer = ArtifactObserver::default();
    let event = SwarmEvent::SwarmHeartbeat {
        session_id: None,
        cluster_id: None,
        agent_id: None,
        role_class: None,
        cpu_percent: None,
        memory_bytes: None,
        timestamp_ms: 0,
    };
    assert!(!observer.should_handle_event(&event));
}

#[tokio::test]
async fn observer_ingest_artifact_success() {
    let observer = ArtifactObserver::default();
    let trace = CognitiveTraceRecord::new(
        "trace-ingest-1".to_string(),
        Some("session-1".to_string()),
        "AuditNode".to_string(),
        "Critique the agenda".to_string(),
    );
    let result = observer.ingest_artifact(&trace).await;
    match result {
        ArtifactIngestionResult::Ingested {
            trace_id,
            anchor_id,
        } => {
            assert_eq!(trace_id, "trace-ingest-1");
            assert_eq!(anchor_id, "noop:trace-ingest-1");
        }
        _ => panic!("expected Ingested variant, got {result:?}"),
    }
}

#[tokio::test]
async fn observer_ingest_disabled_returns_skipped() {
    let config = ArtifactObserverConfig {
        enabled: false,
        ..Default::default()
    };
    let observer = ArtifactObserver::new(config, NoopWendaoIngestionSink);
    let trace = CognitiveTraceRecord::new(
        "trace-disabled".to_string(),
        None,
        "TestNode".to_string(),
        "Test".to_string(),
    );
    let result = observer.ingest_artifact(&trace).await;
    match result {
        ArtifactIngestionResult::Skipped { reason } => {
            assert_eq!(reason.as_ref(), "ingestion disabled");
        }
        _ => panic!("expected Skipped variant"),
    }
}

#[tokio::test]
async fn observer_ingest_early_halt_skipped_when_disabled() {
    let config = ArtifactObserverConfig {
        ingest_on_early_halt: false,
        ..Default::default()
    };
    let observer = ArtifactObserver::new(config, NoopWendaoIngestionSink);
    let mut trace = CognitiveTraceRecord::new(
        "trace-halt".to_string(),
        None,
        "MonitorNode".to_string(),
        "Monitor".to_string(),
    );
    trace.early_halt_triggered = true;
    let result = observer.ingest_artifact(&trace).await;
    match result {
        ArtifactIngestionResult::Skipped { reason } => {
            assert_eq!(reason.as_ref(), "early halt ingestion disabled");
        }
        _ => panic!("expected Skipped variant"),
    }
}

#[tokio::test]
async fn observer_ingest_early_halt_allowed_when_enabled() {
    let observer = ArtifactObserver::default();
    let mut trace = CognitiveTraceRecord::new(
        "trace-halt-enabled".to_string(),
        None,
        "MonitorNode".to_string(),
        "Monitor".to_string(),
    );
    trace.early_halt_triggered = true;
    let result = observer.ingest_artifact(&trace).await;
    match result {
        ArtifactIngestionResult::Ingested { trace_id, .. } => {
            assert_eq!(trace_id, "trace-halt-enabled");
        }
        _ => panic!("expected Ingested variant"),
    }
}

#[test]
fn builder_creates_default_observer() {
    let observer = ArtifactObserverBuilder::new().build_noop();
    assert!(observer.config().enabled);
}

#[test]
fn builder_disabled() {
    let observer = ArtifactObserverBuilder::new().enabled(false).build_noop();
    assert!(!observer.config().enabled);
}

#[test]
fn builder_custom_trace_path() {
    let observer = ArtifactObserverBuilder::new()
        .trace_base_path("custom/traces")
        .build_noop();
    assert_eq!(observer.config().trace_base_path, "custom/traces");
}

#[test]
fn builder_ingest_on_exit_false() {
    let observer = ArtifactObserverBuilder::new()
        .ingest_on_exit(false)
        .build_noop();
    assert!(!observer.config().ingest_on_exit);
}

#[test]
fn builder_ingest_on_early_halt_false() {
    let observer = ArtifactObserverBuilder::new()
        .ingest_on_early_halt(false)
        .build_noop();
    assert!(!observer.config().ingest_on_early_halt);
}

#[test]
fn builder_chained_config() {
    let observer = ArtifactObserverBuilder::new()
        .enabled(false)
        .trace_base_path("my/path")
        .ingest_on_exit(false)
        .ingest_on_early_halt(false)
        .build_noop();
    let config = observer.config();
    assert!(!config.enabled);
    assert_eq!(config.trace_base_path, "my/path");
    assert!(!config.ingest_on_exit);
    assert!(!config.ingest_on_early_halt);
}

#[derive(Debug, Default)]
struct MockIngestionSink {
    call_count: AtomicUsize,
    last_trace_id: std::sync::Mutex<Option<String>>,
}

#[async_trait]
impl WendaoIngestionSink for MockIngestionSink {
    async fn ingest_trace(
        &self,
        trace: &CognitiveTraceRecord,
        _document: &LinkGraphSemanticDocument,
    ) -> Result<String, String> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        let mut last = self
            .last_trace_id
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *last = Some(trace.trace_id.clone());
        Ok(format!("mock:{}", trace.trace_id))
    }
}

#[tokio::test]
async fn observer_with_mock_sink() {
    let sink = MockIngestionSink::default();
    let observer = ArtifactObserverBuilder::new().sink(sink).build();

    let trace = CognitiveTraceRecord::new(
        "trace-mock".to_string(),
        None,
        "TestNode".to_string(),
        "Test".to_string(),
    );

    let result = observer.ingest_artifact(&trace).await;
    match result {
        ArtifactIngestionResult::Ingested {
            trace_id,
            anchor_id,
        } => {
            assert_eq!(trace_id, "trace-mock");
            assert_eq!(anchor_id, "mock:trace-mock");
        }
        _ => panic!("expected Ingested variant"),
    }
}

#[tokio::test]
async fn observer_debug_format() {
    let observer = ArtifactObserver::default();
    let debug_str = format!("{observer:?}");
    assert!(debug_str.contains("ArtifactObserver"));
}

#[test]
fn observer_config_access() {
    let config = ArtifactObserverConfig {
        enabled: false,
        trace_base_path: "test/path".to_string(),
        ingest_on_exit: true,
        ingest_on_early_halt: false,
    };
    let observer = ArtifactObserver::new(config.clone(), NoopWendaoIngestionSink);
    let observed_config = observer.config();
    assert!(!observed_config.enabled);
    assert_eq!(observed_config.trace_base_path, "test/path");
    assert!(observed_config.ingest_on_exit);
    assert!(!observed_config.ingest_on_early_halt);
}
