//! Wendao Ingestion Sink Implementations (Blueprint V6.1).
//!
//! Provides concrete implementations of `WendaoIngestionSink` for persisting
//! cognitive traces to Wendao-compatible storage.
//!
//! ## Architecture
//!
//! ```text
//! ArtifactObserver
//!        │
//!        ▼ WendaoIngestionSink::ingest_trace()
//! FileWendaoSink
//!        │
//!        ▼ Write markdown file to .cognitive/traces/
//! Wendao LinkGraphIndex (on next rebuild)
//!        │
//!        ▼ CognitiveTrace queryable via Wendao
//! ```

use super::artifact_observer::WendaoIngestionSink;
use async_trait::async_trait;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use xiuxian_wendao::link_graph::{CognitiveTraceRecord, LinkGraphSemanticDocument};

/// File-based Wendao ingestion sink.
///
/// Writes cognitive traces as markdown files to a configured directory,
/// which are then indexed by Wendao's `LinkGraphIndex` on the next rebuild.
#[derive(Debug, Clone)]
pub struct FileWendaoSink {
    /// Base directory for trace files.
    base_dir: PathBuf,
    /// Whether to create the directory if it doesn't exist.
    create_dir: bool,
}

impl FileWendaoSink {
    /// Create a new file-based sink with the given base directory.
    #[must_use]
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            create_dir: true,
        }
    }

    /// Create a sink that won't create directories automatically.
    #[must_use]
    pub fn new_no_create(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
            create_dir: false,
        }
    }

    /// Get the base directory for trace files.
    #[must_use]
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Generate the file path for a cognitive trace.
    fn trace_path(&self, trace: &CognitiveTraceRecord) -> PathBuf {
        let filename = format!("{}.md", trace.trace_id.replace(':', "-"));
        self.base_dir.join(filename)
    }

    /// Render a cognitive trace as markdown.
    fn render_markdown(trace: &CognitiveTraceRecord) -> String {
        let mut md = String::new();
        let trace_id = &trace.trace_id;
        let node_id = &trace.node_id;

        // YAML frontmatter for metadata
        md.push_str("---\n");
        writeln!(md, "trace_id: {trace_id}")
            .unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
        if let Some(ref session_id) = trace.session_id {
            writeln!(md, "session_id: {session_id}")
                .unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
        }
        writeln!(md, "node_id: {node_id}")
            .unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
        writeln!(md, "timestamp_ms: {}", trace.timestamp_ms)
            .unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
        if let Some(score) = trace.coherence_score {
            writeln!(md, "coherence_score: {score:.2}")
                .unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
        }
        if trace.early_halt_triggered {
            md.push_str("early_halt_triggered: true\n");
        }
        if let Some(ref sha) = trace.commit_sha {
            writeln!(md, "commit_sha: {sha}")
                .unwrap_or_else(|_| unreachable!("writing to String cannot fail"));
        }
        md.push_str("---\n\n");

        // Title
        writeln!(md, "# Cognitive Trace: {node_id}\n")
            .unwrap_or_else(|_| unreachable!("writing to String cannot fail"));

        // Intent section
        md.push_str("## Intent\n\n");
        md.push_str(&trace.intent);
        md.push_str("\n\n");

        // Reasoning section
        md.push_str("## Reasoning\n\n");
        md.push_str(&trace.reasoning);
        md.push_str("\n\n");

        // Outcome section (if present)
        if let Some(ref outcome) = trace.outcome {
            md.push_str("## Outcome\n\n");
            md.push_str(outcome);
            md.push_str("\n\n");
        }

        md
    }
}

#[async_trait]
impl WendaoIngestionSink for FileWendaoSink {
    async fn ingest_trace(
        &self,
        trace: &CognitiveTraceRecord,
        _document: &LinkGraphSemanticDocument,
    ) -> Result<String, String> {
        let path = self.trace_path(trace);

        // Create directory if needed
        if self.create_dir
            && let Err(e) = tokio::fs::create_dir_all(&self.base_dir).await
        {
            return Err(format!("Failed to create trace directory: {e}"));
        }

        // Render markdown
        let content = Self::render_markdown(trace);

        // Write file
        if let Err(e) = tokio::fs::write(&path, content).await {
            return Err(format!("Failed to write trace file: {e}"));
        }

        Ok(format!("file:{}", path.display()))
    }
}

/// In-memory Wendao ingestion sink for testing.
///
/// Stores traces in memory without persisting to disk.
#[derive(Debug, Default)]
pub struct InMemoryWendaoSink {
    /// Stored traces.
    traces: Mutex<Vec<(CognitiveTraceRecord, String)>>,
}

impl InMemoryWendaoSink {
    /// Create a new in-memory sink.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn traces_guard(&self) -> MutexGuard<'_, Vec<(CognitiveTraceRecord, String)>> {
        self.traces.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Get all stored traces.
    pub fn traces(&self) -> Vec<(CognitiveTraceRecord, String)> {
        self.traces_guard().clone()
    }

    /// Get the number of stored traces.
    pub fn len(&self) -> usize {
        self.traces_guard().len()
    }

    /// Check if no traces are stored.
    pub fn is_empty(&self) -> bool {
        self.traces_guard().is_empty()
    }

    /// Clear all stored traces.
    pub fn clear(&self) {
        self.traces_guard().clear();
    }
}

#[async_trait]
impl WendaoIngestionSink for InMemoryWendaoSink {
    async fn ingest_trace(
        &self,
        trace: &CognitiveTraceRecord,
        _document: &LinkGraphSemanticDocument,
    ) -> Result<String, String> {
        let anchor_id = format!("memory:{}", trace.trace_id);
        self.traces_guard().push((trace.clone(), anchor_id.clone()));
        Ok(anchor_id)
    }
}

/// Composite sink that tries multiple sinks in sequence.
#[derive(Debug)]
pub struct CompositeWendaoSink {
    /// Primary sink to try first.
    primary: Arc<dyn WendaoIngestionSink>,
    /// Fallback sink if primary fails.
    fallback: Option<Arc<dyn WendaoIngestionSink>>,
}

impl CompositeWendaoSink {
    /// Create a new composite sink with a primary and optional fallback.
    #[must_use]
    pub fn new(
        primary: Arc<dyn WendaoIngestionSink>,
        fallback: Option<Arc<dyn WendaoIngestionSink>>,
    ) -> Self {
        Self { primary, fallback }
    }

    /// Create a builder for constructing a composite sink.
    #[must_use]
    pub fn builder() -> CompositeWendaoSinkBuilder {
        CompositeWendaoSinkBuilder::default()
    }
}

#[async_trait]
impl WendaoIngestionSink for CompositeWendaoSink {
    async fn ingest_trace(
        &self,
        trace: &CognitiveTraceRecord,
        document: &LinkGraphSemanticDocument,
    ) -> Result<String, String> {
        match self.primary.ingest_trace(trace, document).await {
            Ok(anchor_id) => Ok(anchor_id),
            Err(primary_error) => {
                if let Some(ref fallback) = self.fallback {
                    fallback
                        .ingest_trace(trace, document)
                        .await
                        .map_err(|fallback_error| {
                            format!(
                                "Primary failed: {primary_error}; Fallback failed: {fallback_error}"
                            )
                        })
                } else {
                    Err(primary_error)
                }
            }
        }
    }
}

/// Builder for composite sink configuration.
#[derive(Debug, Default)]
pub struct CompositeWendaoSinkBuilder {
    primary: Option<Arc<dyn WendaoIngestionSink>>,
    fallback: Option<Arc<dyn WendaoIngestionSink>>,
}

impl CompositeWendaoSinkBuilder {
    /// Set the primary sink.
    #[must_use]
    pub fn primary(mut self, sink: Arc<dyn WendaoIngestionSink>) -> Self {
        self.primary = Some(sink);
        self
    }

    /// Set the fallback sink.
    #[must_use]
    pub fn fallback(mut self, sink: Arc<dyn WendaoIngestionSink>) -> Self {
        self.fallback = Some(sink);
        self
    }

    /// Build the composite sink.
    ///
    /// # Panics
    ///
    /// Panics if no primary sink was configured.
    #[must_use]
    pub fn build(self) -> CompositeWendaoSink {
        let Some(primary) = self.primary else {
            panic!("primary sink must be configured");
        };
        CompositeWendaoSink::new(primary, self.fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // === FileWendaoSink Tests ===

    #[test]
    fn file_sink_new_creates_base_dir() {
        let sink = FileWendaoSink::new("/tmp/traces");
        assert_eq!(sink.base_dir(), Path::new("/tmp/traces"));
        assert!(sink.create_dir);
    }

    #[test]
    fn file_sink_new_no_create() {
        let sink = FileWendaoSink::new_no_create("/tmp/traces");
        assert!(!sink.create_dir);
    }

    #[test]
    fn file_sink_clone_preserves_settings() {
        let sink = FileWendaoSink::new("/tmp/traces");
        let cloned = sink.clone();
        assert_eq!(sink.base_dir(), cloned.base_dir());
        assert_eq!(sink.create_dir, cloned.create_dir);
    }

    #[tokio::test]
    async fn file_sink_writes_trace_file() {
        let temp_dir =
            TempDir::new().unwrap_or_else(|err| panic!("failed to create temp dir: {err}"));
        let sink = FileWendaoSink::new(temp_dir.path());

        let trace = CognitiveTraceRecord::new(
            "trace-test-123".to_string(),
            Some("session-abc".to_string()),
            "AuditNode".to_string(),
            "Critique the agenda".to_string(),
        );

        let doc = trace.to_semantic_document("doc-1", "traces/test.md");
        let result = sink.ingest_trace(&trace, &doc).await;

        assert!(matches!(
            result.as_deref(),
            Ok(anchor_id)
                if anchor_id.starts_with("file:") && anchor_id.contains("trace-test-123.md")
        ));
    }

    #[tokio::test]
    async fn file_sink_creates_directory() {
        let temp_dir =
            TempDir::new().unwrap_or_else(|err| panic!("failed to create temp dir: {err}"));
        let nested_path = temp_dir.path().join("nested").join("dir");
        let sink = FileWendaoSink::new(&nested_path);

        let trace = CognitiveTraceRecord::new(
            "trace-dir-test".to_string(),
            None,
            "TestNode".to_string(),
            "Test".to_string(),
        );

        let doc = trace.to_semantic_document("doc-2", "test.md");
        let result = sink.ingest_trace(&trace, &doc).await;

        assert!(result.is_ok());
        assert!(nested_path.exists());
    }

    #[tokio::test]
    async fn file_sink_produces_valid_markdown() {
        let temp_dir =
            TempDir::new().unwrap_or_else(|err| panic!("failed to create temp dir: {err}"));
        let sink = FileWendaoSink::new(temp_dir.path());

        let trace = CognitiveTraceRecord::new(
            "trace-md-test".to_string(),
            Some("session-xyz".to_string()),
            "PlanNode".to_string(),
            "Generate a plan".to_string(),
        );

        let doc = trace.to_semantic_document("doc-3", "test.md");
        sink.ingest_trace(&trace, &doc)
            .await
            .unwrap_or_else(|err| panic!("failed to ingest trace: {err}"));

        let file_path = temp_dir.path().join("trace-md-test.md");
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .unwrap_or_else(|err| panic!("failed to read markdown output: {err}"));

        assert!(content.contains("---"));
        assert!(content.contains("trace_id: trace-md-test"));
        assert!(content.contains("session_id: session-xyz"));
        assert!(content.contains("node_id: PlanNode"));
        assert!(content.contains("# Cognitive Trace: PlanNode"));
        assert!(content.contains("## Intent"));
        assert!(content.contains("Generate a plan"));
        assert!(content.contains("## Reasoning"));
    }

    #[tokio::test]
    async fn file_sink_includes_outcome() {
        let temp_dir =
            TempDir::new().unwrap_or_else(|err| panic!("failed to create temp dir: {err}"));
        let sink = FileWendaoSink::new(temp_dir.path());

        let mut trace = CognitiveTraceRecord::new(
            "trace-outcome".to_string(),
            None,
            "TestNode".to_string(),
            "Test".to_string(),
        );
        trace.outcome = Some(Arc::<str>::from("Task completed successfully"));

        let doc = trace.to_semantic_document("doc-4", "test.md");
        sink.ingest_trace(&trace, &doc)
            .await
            .unwrap_or_else(|err| panic!("failed to ingest trace: {err}"));

        let file_path = temp_dir.path().join("trace-outcome.md");
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .unwrap_or_else(|err| panic!("failed to read markdown output: {err}"));

        assert!(content.contains("## Outcome"));
        assert!(content.contains("Task completed successfully"));
    }

    #[tokio::test]
    async fn file_sink_includes_coherence_score() {
        let temp_dir =
            TempDir::new().unwrap_or_else(|err| panic!("failed to create temp dir: {err}"));
        let sink = FileWendaoSink::new(temp_dir.path());

        let mut trace = CognitiveTraceRecord::new(
            "trace-coherence".to_string(),
            None,
            "MonitorNode".to_string(),
            "Monitor".to_string(),
        );
        trace.coherence_score = Some(0.85);
        trace.early_halt_triggered = true;

        let doc = trace.to_semantic_document("doc-5", "test.md");
        sink.ingest_trace(&trace, &doc)
            .await
            .unwrap_or_else(|err| panic!("failed to ingest trace: {err}"));

        let file_path = temp_dir.path().join("trace-coherence.md");
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .unwrap_or_else(|err| panic!("failed to read markdown output: {err}"));

        assert!(content.contains("coherence_score: 0.85"));
        assert!(content.contains("early_halt_triggered: true"));
    }

    // === InMemoryWendaoSink Tests ===

    #[test]
    fn memory_sink_new_creates_empty() {
        let sink = InMemoryWendaoSink::new();
        assert!(sink.is_empty());
        assert_eq!(sink.len(), 0);
    }

    #[tokio::test]
    async fn memory_sink_stores_trace() {
        let sink = InMemoryWendaoSink::new();

        let trace = CognitiveTraceRecord::new(
            "trace-mem-1".to_string(),
            None,
            "TestNode".to_string(),
            "Test".to_string(),
        );

        let doc = trace.to_semantic_document("doc-m1", "test.md");
        let result = sink.ingest_trace(&trace, &doc).await;

        assert_eq!(result.as_deref(), Ok("memory:trace-mem-1"));
        assert_eq!(sink.len(), 1);
    }

    #[tokio::test]
    async fn memory_sink_multiple_traces() {
        let sink = InMemoryWendaoSink::new();

        for i in 0..3 {
            let trace = CognitiveTraceRecord::new(
                format!("trace-mem-{i}"),
                None,
                "TestNode".to_string(),
                "Test".to_string(),
            );
            let doc = trace.to_semantic_document(&format!("doc-{i}"), "test.md");
            sink.ingest_trace(&trace, &doc)
                .await
                .unwrap_or_else(|err| panic!("failed to ingest trace: {err}"));
        }

        assert_eq!(sink.len(), 3);

        let traces = sink.traces();
        assert_eq!(traces.len(), 3);
    }

    #[tokio::test]
    async fn memory_sink_clear() {
        let sink = InMemoryWendaoSink::new();

        let trace = CognitiveTraceRecord::new(
            "trace-clear".to_string(),
            None,
            "TestNode".to_string(),
            "Test".to_string(),
        );
        let doc = trace.to_semantic_document("doc-c", "test.md");
        sink.ingest_trace(&trace, &doc)
            .await
            .unwrap_or_else(|err| panic!("failed to ingest trace: {err}"));

        assert_eq!(sink.len(), 1);

        sink.clear();
        assert!(sink.is_empty());
    }

    // === CompositeWendaoSink Tests ===

    #[tokio::test]
    async fn composite_sink_uses_primary() {
        let primary = Arc::new(InMemoryWendaoSink::new());
        let fallback = Arc::new(InMemoryWendaoSink::new());

        let sink = CompositeWendaoSink::builder()
            .primary(primary.clone())
            .fallback(fallback.clone())
            .build();

        let trace = CognitiveTraceRecord::new(
            "trace-comp-1".to_string(),
            None,
            "TestNode".to_string(),
            "Test".to_string(),
        );
        let doc = trace.to_semantic_document("doc-c1", "test.md");

        let result = sink.ingest_trace(&trace, &doc).await;
        assert!(result.is_ok());
        assert_eq!(primary.len(), 1);
        assert_eq!(fallback.len(), 0);
    }

    #[test]
    fn composite_sink_builder_requires_primary() {
        let result = std::panic::catch_unwind(|| {
            let _sink = CompositeWendaoSink::builder().build();
        });
        assert!(result.is_err());
    }

    // === FileWendaoSink Render Tests ===

    #[test]
    fn render_markdown_minimal() {
        let trace = CognitiveTraceRecord::new(
            "trace-render-min".to_string(),
            None,
            "TestNode".to_string(),
            "Test intent".to_string(),
        );

        let md = FileWendaoSink::render_markdown(&trace);

        assert!(md.contains("trace_id: trace-render-min"));
        assert!(md.contains("node_id: TestNode"));
        assert!(!md.contains("session_id:"));
        assert!(!md.contains("coherence_score:"));
    }

    #[test]
    fn render_markdown_full() {
        let mut trace = CognitiveTraceRecord::new(
            "trace-render-full".to_string(),
            Some("session-123".to_string()),
            "FullNode".to_string(),
            "Full test".to_string(),
        );
        trace.coherence_score = Some(0.92);
        trace.early_halt_triggered = false;
        trace.commit_sha = Some("abc123def".to_string());
        trace.outcome = Some(Arc::<str>::from("Success"));

        let md = FileWendaoSink::render_markdown(&trace);

        assert!(md.contains("session_id: session-123"));
        assert!(md.contains("coherence_score: 0.92"));
        assert!(md.contains("commit_sha: abc123def"));
        assert!(md.contains("## Outcome"));
        assert!(md.contains("Success"));
    }

    #[test]
    fn render_markdown_early_halt() {
        let mut trace = CognitiveTraceRecord::new(
            "trace-halt".to_string(),
            None,
            "HaltNode".to_string(),
            "Test".to_string(),
        );
        trace.early_halt_triggered = true;
        trace.coherence_score = Some(0.15);

        let md = FileWendaoSink::render_markdown(&trace);

        assert!(md.contains("early_halt_triggered: true"));
        assert!(md.contains("coherence_score: 0.15"));
    }
}
