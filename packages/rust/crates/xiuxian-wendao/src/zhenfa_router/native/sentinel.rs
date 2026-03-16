//! Project Sentinel: Real-time synchronization and semantic change propagation.
//!
//! This module provides the infrastructure for observing the filesystem and
//! automatically updating the LinkGraph and Audit reports when files change.
//!
//! ## Phase 6: Semantic Change Propagation
//!
//! When source code changes, Sentinel identifies "Observational Casualties" -
//! documents with `:OBSERVE:` patterns that may reference the changed code.
//! These are surfaced as `SemanticDriftSignal` events for agent notification.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use chrono;
use log::{error, info};
use notify::{Event, RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, FileIdMap, new_debouncer};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use xiuxian_zhenfa::ZhenfaContext;

use crate::LinkGraphIndex;
use crate::link_graph::PageIndexNode;
use crate::zhenfa_router::native::WendaoContextExt;

/// Configuration for the Sentinel observer.
#[derive(Debug, Clone)]
pub struct SentinelConfig {
    /// Paths to watch for changes.
    pub watch_paths: Vec<PathBuf>,
    /// Debounce duration (increased for CAS consistency).
    pub debounce_duration: Duration,
}

impl Default for SentinelConfig {
    fn default() -> Self {
        Self {
            watch_paths: vec![PathBuf::from("docs"), PathBuf::from("src")],
            // Increased to 1000ms for CAS consistency (audit recommendation)
            debounce_duration: Duration::from_millis(1000),
        }
    }
}

/// The Sentinel observer.
pub struct Sentinel {
    ctx: Arc<ZhenfaContext>,
    config: SentinelConfig,
    _debouncer: Debouncer<RecommendedWatcher, FileIdMap>,
}

impl Sentinel {
    /// Create and start a new Sentinel observer.
    pub async fn start(
        ctx: Arc<ZhenfaContext>,
        config: SentinelConfig,
    ) -> Result<Self, anyhow::Error> {
        let (tx, mut rx) = mpsc::channel(100);

        // Create the debouncer
        // DebounceEventResult = Result<Vec<DebouncedEvent>, Vec<Error>>
        let mut debouncer = new_debouncer(
            config.debounce_duration,
            None,
            move |result: DebounceEventResult| {
                if let Ok(events) = result {
                    for event in events {
                        let _ = tx.try_send(event.event);
                    }
                }
            },
        )?;

        // Watch the paths - new API uses debouncer.watch() directly
        for path in &config.watch_paths {
            if path.exists() {
                info!("Sentinel watching: {:?}", path);
                debouncer.watch(path, RecursiveMode::Recursive)?;
            }
        }

        // Spawn the event handler
        let handler_ctx = ctx.clone();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                if let Err(e) = handle_sentinel_event(&handler_ctx, event).await {
                    error!("Sentinel event handler error: {:?}", e);
                }
            }
        });

        Ok(Self {
            ctx,
            config,
            _debouncer: debouncer,
        })
    }
}

/// Internal event handler for Sentinel.
async fn handle_sentinel_event(ctx: &ZhenfaContext, event: Event) -> Result<(), anyhow::Error> {
    for path in event.paths {
        if is_ignorable_path(&path) {
            continue;
        }

        info!("Sentinel detected change in: {:?}", path);

        // PHASE 5: Instant LinkGraph Refresh
        // TODO: Implement incremental indexing for modified docs.

        // PHASE 6: Semantic Change Propagation
        if is_source_code(&path) {
            if let Ok(index) = ctx.link_graph_index() {
                let signals = propagate_source_change(&index, &path);
                if !signals.is_empty() {
                    info!(
                        "Phase 6.2: Generated {} semantic drift signal(s)",
                        signals.len()
                    );
                    // TODO: Stream signals to agent via ZhenfaStreamingEvent bus
                    for signal in &signals {
                        info!("  Signal: {}", signal.summary());
                    }
                }
            }
        }
    }
    Ok(())
}

fn is_ignorable_path(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains(".git") || s.contains("target") || s.contains(".gemini")
}

fn is_source_code(path: &Path) -> bool {
    path.extension().map_or(false, |ext| {
        ext == "rs" || ext == "py" || ext == "ts" || ext == "js"
    })
}

// =============================================================================
// Phase 6.2: Semantic Drift Signal Types
// =============================================================================

/// A signal indicating that source code changes may affect documentation.
///
/// This struct captures the relationship between a changed source file and
/// documents that contain `:OBSERVE:` patterns potentially referencing it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticDriftSignal {
    /// The source file that changed.
    pub source_path: String,
    /// File stem used for heuristic matching.
    pub file_stem: String,
    /// Documents with observations that may reference this source.
    pub affected_docs: Vec<AffectedDoc>,
    /// Confidence level of the drift detection.
    pub confidence: DriftConfidence,
    /// Timestamp of the detection.
    pub timestamp: String,
}

/// A document potentially affected by source code changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedDoc {
    /// Document ID (stem or full path).
    pub doc_id: String,
    /// The observation pattern that matched the source file.
    pub matching_pattern: String,
    /// Language of the observation.
    pub language: String,
    /// Line number of the observation in the document.
    pub line_number: Option<usize>,
    /// Node ID where the observation was found.
    pub node_id: String,
}

/// Confidence level for drift detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DriftConfidence {
    /// High confidence: pattern explicitly references the file/symbol.
    High,
    /// Medium confidence: pattern contains related keywords.
    Medium,
    /// Low confidence: fuzzy heuristic match only.
    Low,
}

impl SemanticDriftSignal {
    /// Create a new semantic drift signal.
    #[must_use]
    pub fn new(source_path: impl Into<String>, file_stem: impl Into<String>) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339();
        Self {
            source_path: source_path.into(),
            file_stem: file_stem.into(),
            affected_docs: Vec::new(),
            confidence: DriftConfidence::Low,
            timestamp,
        }
    }

    /// Add an affected document to the signal.
    pub fn add_affected_doc(&mut self, doc: AffectedDoc) {
        self.affected_docs.push(doc);
    }

    /// Update confidence based on match quality.
    pub fn update_confidence(&mut self, confidence: DriftConfidence) {
        self.confidence = confidence;
    }

    /// Generate a human-readable summary.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Semantic drift in '{}' may affect {} doc(s): {}",
            self.file_stem,
            self.affected_docs.len(),
            self.affected_docs
                .iter()
                .map(|d| d.doc_id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    /// Convert to streaming event payload.
    #[must_use]
    pub fn to_streaming_payload(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

impl AffectedDoc {
    /// Create a new affected document record.
    #[must_use]
    pub fn new(
        doc_id: impl Into<String>,
        matching_pattern: impl Into<String>,
        language: impl Into<String>,
        node_id: impl Into<String>,
    ) -> Self {
        Self {
            doc_id: doc_id.into(),
            matching_pattern: matching_pattern.into(),
            language: language.into(),
            line_number: None,
            node_id: node_id.into(),
        }
    }

    /// Set the line number.
    #[must_use]
    pub fn with_line(mut self, line: usize) -> Self {
        self.line_number = Some(line);
        self
    }
}

// =============================================================================
// Phase 6: Core Propagation Logic
// =============================================================================

/// Phase 6: Core logic for propagating source changes to documentation.
///
/// Scans all documents with `:OBSERVE:` patterns and identifies those that
/// may reference the changed source file using heuristic matching.
///
/// # Returns
///
/// A vector of `SemanticDriftSignal` events for each affected observation.
#[must_use]
pub fn propagate_source_change(index: &LinkGraphIndex, path: &Path) -> Vec<SemanticDriftSignal> {
    info!("Propagating semantic change from code: {:?}", path);

    let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let file_stem_lower = file_stem.to_lowercase();

    let mut signal = SemanticDriftSignal::new(path.to_string_lossy(), file_stem);
    let trees = index.all_page_index_trees();

    // Traverse all document trees to find observations
    for (doc_id, nodes) in trees {
        traverse_nodes_for_observations(&nodes, doc_id, file_stem, &file_stem_lower, &mut signal);
    }

    if signal.affected_docs.is_empty() {
        return Vec::new();
    }

    // Determine confidence based on match quality
    let has_explicit_reference = signal
        .affected_docs
        .iter()
        .any(|d| d.matching_pattern.contains(&format!("fn {}", file_stem)))
        || signal.affected_docs.iter().any(|d| {
            d.matching_pattern
                .contains(&format!("struct {}", file_stem))
        })
        || signal
            .affected_docs
            .iter()
            .any(|d| d.matching_pattern.contains(&format!("class {}", file_stem)));

    signal.update_confidence(if has_explicit_reference {
        DriftConfidence::High
    } else if signal.affected_docs.len() <= 3 {
        DriftConfidence::Medium
    } else {
        DriftConfidence::Low
    });

    info!(
        "Phase 6: {} documents potentially affected by source change.",
        signal.affected_docs.len()
    );

    vec![signal]
}

/// Recursively traverse page index nodes to find matching observations.
fn traverse_nodes_for_observations(
    nodes: &[PageIndexNode],
    doc_id: &str,
    file_stem: &str,
    file_stem_lower: &str,
    signal: &mut SemanticDriftSignal,
) {
    for node in nodes {
        // Check observations in this node's metadata
        for obs in &node.metadata.observations {
            let pattern_lower = obs.pattern.to_lowercase();

            // Heuristic matching: pattern contains file stem or related symbols
            let matches = pattern_lower.contains(file_stem_lower)
                || obs
                    .pattern
                    .contains(&format!("{}_{}", file_stem, file_stem))
                || obs.pattern.contains(&format!("{}::", file_stem))
                || obs.pattern.contains(&format!("{}.", file_stem));

            if matches {
                let affected = AffectedDoc::new(
                    doc_id,
                    obs.pattern.clone(),
                    obs.language.clone(),
                    node.node_id.clone(),
                )
                .with_line(obs.line_number.unwrap_or(node.metadata.line_range.0));

                signal.add_affected_doc(affected);
            }
        }

        // Recurse into children
        traverse_nodes_for_observations(&node.children, doc_id, file_stem, file_stem_lower, signal);
    }
}

// =============================================================================
// Phase 6.2: Observation Signal Types for Agent Integration
// =============================================================================

/// Signal types for observation lifecycle events.
///
/// These signals are emitted when code observations need attention:
/// - `Stale`: The observed code may have changed, observation needs re-validation
/// - `Broken`: The observed code structure no longer matches the pattern
/// - `Orphaned`: The source file referenced by the observation no longer exists
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObservationSignal {
    /// Observation pattern may be outdated due to source changes.
    Stale {
        /// Document containing the observation.
        doc_id: String,
        /// The observation pattern that may need updating.
        observation: ObservationRef,
        /// Source file that triggered the stale signal.
        trigger_source: String,
        /// Confidence that this observation is affected.
        confidence: DriftConfidence,
    },
    /// Observation pattern no longer matches any code structure.
    Broken {
        /// Document containing the broken observation.
        doc_id: String,
        /// The broken observation pattern.
        observation: ObservationRef,
        /// Error message describing the breakage.
        error: String,
    },
    /// Source file referenced by observation no longer exists.
    Orphaned {
        /// Document containing the orphaned observation.
        doc_id: String,
        /// The orphaned observation pattern.
        observation: ObservationRef,
        /// Former source file location.
        former_source: String,
    },
}

/// Reference to a code observation within a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationRef {
    /// The observation pattern (sgrep/ast-grep syntax).
    pub pattern: String,
    /// Target language.
    pub language: String,
    /// Line number in the document.
    pub line_number: usize,
    /// Node ID where the observation is located.
    pub node_id: String,
}

impl ObservationSignal {
    /// Create a stale signal from a semantic drift detection.
    #[must_use]
    pub fn stale_from_drift(drift: &SemanticDriftSignal) -> Vec<Self> {
        drift
            .affected_docs
            .iter()
            .map(|doc| Self::Stale {
                doc_id: doc.doc_id.clone(),
                observation: ObservationRef {
                    pattern: doc.matching_pattern.clone(),
                    language: doc.language.clone(),
                    line_number: doc.line_number.unwrap_or(0),
                    node_id: doc.node_id.clone(),
                },
                trigger_source: drift.source_path.clone(),
                confidence: drift.confidence,
            })
            .collect()
    }

    /// Convert signal to a streaming-friendly status message.
    #[must_use]
    pub fn to_status_message(&self) -> String {
        match self {
            Self::Stale {
                doc_id,
                observation,
                trigger_source,
                confidence,
            } => {
                format!(
                    "⚠️ Stale observation in {}: '{}' may need update (triggered by {}, {:?} confidence)",
                    doc_id, observation.pattern, trigger_source, confidence
                )
            }
            Self::Broken {
                doc_id,
                observation,
                error,
            } => {
                format!(
                    "❌ Broken observation in {}: '{}' - {}",
                    doc_id, observation.pattern, error
                )
            }
            Self::Orphaned {
                doc_id,
                observation,
                former_source,
            } => {
                format!(
                    "���� Orphaned observation in {}: '{}' (source {} no longer exists)",
                    doc_id, observation.pattern, former_source
                )
            }
        }
    }

    /// Get the affected document ID.
    #[must_use]
    pub fn doc_id(&self) -> &str {
        match self {
            Self::Stale { doc_id, .. } => doc_id,
            Self::Broken { doc_id, .. } => doc_id,
            Self::Orphaned { doc_id, .. } => doc_id,
        }
    }

    /// Check if this signal requires immediate attention.
    #[must_use]
    pub fn requires_attention(&self) -> bool {
        matches!(
            self,
            Self::Broken { .. }
                | Self::Stale {
                    confidence: DriftConfidence::High,
                    ..
                }
        )
    }
}

// =============================================================================
// Phase 6.2: Streaming Bus Integration
// =============================================================================

use std::sync::atomic::{AtomicU64, Ordering};

/// Global signal counter for unique IDs.
static SIGNAL_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Streaming bus for observation signals.
///
/// This struct manages the flow of observation signals from Sentinel
/// to agent consumers via an MPSC channel.
pub struct ObservationBus {
    /// Sender for observation signals.
    tx: Option<mpsc::UnboundedSender<ObservationSignal>>,
}

impl Default for ObservationBus {
    fn default() -> Self {
        Self::new()
    }
}

impl ObservationBus {
    /// Create a new observation bus.
    #[must_use]
    pub fn new() -> Self {
        Self { tx: None }
    }

    /// Connect the bus to a receiver channel.
    pub fn connect(&mut self, tx: mpsc::UnboundedSender<ObservationSignal>) {
        self.tx = Some(tx);
    }

    /// Emit a signal to connected consumers.
    ///
    /// Returns the signal ID if successfully emitted.
    pub fn emit(&self, signal: ObservationSignal) -> Option<u64> {
        let tx = self.tx.as_ref()?;
        let signal_id = SIGNAL_COUNTER.fetch_add(1, Ordering::SeqCst);

        if tx.send(signal).is_ok() {
            Some(signal_id)
        } else {
            None
        }
    }

    /// Emit multiple signals from a semantic drift detection.
    pub fn emit_drift_signals(&self, drift: &SemanticDriftSignal) -> Vec<u64> {
        let signals = ObservationSignal::stale_from_drift(drift);
        signals.into_iter().filter_map(|s| self.emit(s)).collect()
    }

    /// Check if the bus is connected.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.tx.is_some()
    }
}

/// Convert observation signals to a streaming status format.
///
/// This function transforms internal signals into a format suitable
/// for agent notification via the ZhenfaStreamingEvent::Status channel.
#[must_use]
pub fn signals_to_status_batch(signals: &[ObservationSignal]) -> String {
    let mut batch = String::new();
    batch.push_str("=== Observation Signal Batch ===\n");

    for (i, signal) in signals.iter().enumerate() {
        batch.push_str(&format!("{}. {}\n", i + 1, signal.to_status_message()));
    }

    batch.push_str(&format!(
        "\nTotal: {} signal(s), {} require immediate attention",
        signals.len(),
        signals.iter().filter(|s| s.requires_attention()).count()
    ));

    batch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_drift_signal_summary() {
        let mut signal = SemanticDriftSignal::new("src/lib.rs", "lib");
        signal.add_affected_doc(AffectedDoc::new(
            "docs/api",
            "fn lib_init($$$)",
            "rust",
            "node-1",
        ));
        signal.update_confidence(DriftConfidence::High);

        let summary = signal.summary();
        assert!(summary.contains("lib"));
        assert!(summary.contains("docs/api"));
    }

    #[test]
    fn test_semantic_drift_signal_serialization() {
        let mut signal = SemanticDriftSignal::new("src/lib.rs", "lib");
        signal.add_affected_doc(AffectedDoc::new(
            "docs/api",
            "fn lib_init($$$)",
            "rust",
            "node-1",
        ));

        let json = signal.to_streaming_payload();
        assert!(json.contains("lib"));
        assert!(json.contains("docs/api"));
    }

    #[test]
    fn test_drift_confidence_levels() {
        assert_eq!(DriftConfidence::High, DriftConfidence::High);
        assert_ne!(DriftConfidence::High, DriftConfidence::Low);
    }

    #[test]
    fn test_affected_doc_builder() {
        let doc = AffectedDoc::new("docs/test", "pattern", "rust", "node-1").with_line(42);

        assert_eq!(doc.doc_id, "docs/test");
        assert_eq!(doc.matching_pattern, "pattern");
        assert_eq!(doc.language, "rust");
        assert_eq!(doc.line_number, Some(42));
        assert_eq!(doc.node_id, "node-1");
    }

    #[test]
    fn test_is_source_code() {
        assert!(is_source_code(Path::new("src/lib.rs")));
        assert!(is_source_code(Path::new("app/main.py")));
        assert!(is_source_code(Path::new("ui/index.ts")));
        assert!(is_source_code(Path::new("web/app.js")));
        assert!(!is_source_code(Path::new("docs/README.md")));
        assert!(!is_source_code(Path::new("config.toml")));
    }

    #[test]
    fn test_is_ignorable_path() {
        assert!(is_ignorable_path(Path::new(".git/config")));
        assert!(is_ignorable_path(Path::new("target/debug/app")));
        assert!(!is_ignorable_path(Path::new("src/lib.rs")));
    }

    // =========================================================================
    // ObservationSignal Tests
    // =========================================================================

    #[test]
    fn test_observation_signal_stale_from_drift() {
        let mut drift = SemanticDriftSignal::new("src/lib.rs", "lib");
        drift.add_affected_doc(AffectedDoc::new(
            "docs/api",
            "fn lib_init($$$)",
            "rust",
            "node-1",
        ));
        drift.update_confidence(DriftConfidence::High);

        let signals = ObservationSignal::stale_from_drift(&drift);
        assert_eq!(signals.len(), 1);

        match &signals[0] {
            ObservationSignal::Stale {
                doc_id,
                observation,
                trigger_source,
                confidence,
            } => {
                assert_eq!(doc_id, "docs/api");
                assert_eq!(observation.pattern, "fn lib_init($$$)");
                assert_eq!(observation.language, "rust");
                assert_eq!(*trigger_source, "src/lib.rs");
                assert_eq!(*confidence, DriftConfidence::High);
            }
            _ => panic!("Expected Stale signal"),
        }
    }

    #[test]
    fn test_observation_signal_to_status_message() {
        let signal = ObservationSignal::Stale {
            doc_id: "docs/api".to_string(),
            observation: ObservationRef {
                pattern: "fn test()".to_string(),
                language: "rust".to_string(),
                line_number: 42,
                node_id: "node-1".to_string(),
            },
            trigger_source: "src/lib.rs".to_string(),
            confidence: DriftConfidence::High,
        };

        let msg = signal.to_status_message();
        assert!(msg.contains("Stale"));
        assert!(msg.contains("docs/api"));
        assert!(msg.contains("fn test()"));
        assert!(msg.contains("High"));
    }

    #[test]
    fn test_observation_signal_requires_attention() {
        let high_stale = ObservationSignal::Stale {
            doc_id: "docs/api".to_string(),
            observation: ObservationRef {
                pattern: "fn test()".to_string(),
                language: "rust".to_string(),
                line_number: 1,
                node_id: "n1".to_string(),
            },
            trigger_source: "src/lib.rs".to_string(),
            confidence: DriftConfidence::High,
        };
        assert!(high_stale.requires_attention());

        let low_stale = ObservationSignal::Stale {
            doc_id: "docs/api".to_string(),
            observation: ObservationRef {
                pattern: "fn test()".to_string(),
                language: "rust".to_string(),
                line_number: 1,
                node_id: "n1".to_string(),
            },
            trigger_source: "src/lib.rs".to_string(),
            confidence: DriftConfidence::Low,
        };
        assert!(!low_stale.requires_attention());

        let broken = ObservationSignal::Broken {
            doc_id: "docs/api".to_string(),
            observation: ObservationRef {
                pattern: "fn test()".to_string(),
                language: "rust".to_string(),
                line_number: 1,
                node_id: "n1".to_string(),
            },
            error: "Pattern not found".to_string(),
        };
        assert!(broken.requires_attention());
    }

    #[test]
    fn test_observation_bus_emit() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut bus = ObservationBus::new();
        assert!(!bus.is_connected());

        bus.connect(tx);
        assert!(bus.is_connected());

        let signal = ObservationSignal::Stale {
            doc_id: "docs/api".to_string(),
            observation: ObservationRef {
                pattern: "fn test()".to_string(),
                language: "rust".to_string(),
                line_number: 1,
                node_id: "n1".to_string(),
            },
            trigger_source: "src/lib.rs".to_string(),
            confidence: DriftConfidence::High,
        };

        let id = bus.emit(signal);
        assert!(id.is_some());

        let received = rx.try_recv();
        assert!(received.is_ok());
    }

    #[test]
    fn test_observation_bus_emit_drift_signals() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut bus = ObservationBus::new();
        bus.connect(tx);

        let mut drift = SemanticDriftSignal::new("src/lib.rs", "lib");
        drift.add_affected_doc(AffectedDoc::new("docs/a", "p1", "rust", "n1"));
        drift.add_affected_doc(AffectedDoc::new("docs/b", "p2", "rust", "n2"));

        let ids = bus.emit_drift_signals(&drift);
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn test_signals_to_status_batch() {
        let signals = vec![
            ObservationSignal::Stale {
                doc_id: "docs/a".to_string(),
                observation: ObservationRef {
                    pattern: "fn a()".to_string(),
                    language: "rust".to_string(),
                    line_number: 1,
                    node_id: "n1".to_string(),
                },
                trigger_source: "src/a.rs".to_string(),
                confidence: DriftConfidence::High,
            },
            ObservationSignal::Broken {
                doc_id: "docs/b".to_string(),
                observation: ObservationRef {
                    pattern: "fn b()".to_string(),
                    language: "rust".to_string(),
                    line_number: 2,
                    node_id: "n2".to_string(),
                },
                error: "Not found".to_string(),
            },
        ];

        let batch = signals_to_status_batch(&signals);
        assert!(batch.contains("Observation Signal Batch"));
        assert!(batch.contains("2 signal(s)"));
        assert!(batch.contains("2 require immediate attention"));
    }
}
