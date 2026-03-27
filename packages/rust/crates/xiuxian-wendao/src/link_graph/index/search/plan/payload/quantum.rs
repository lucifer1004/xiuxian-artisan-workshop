use super::super::super::LinkGraphIndex;
#[cfg(feature = "julia")]
use crate::analyzers::languages::validate_julia_arrow_response_batches;
#[cfg(feature = "julia")]
use crate::analyzers::{JuliaArrowScoreRow, decode_julia_arrow_score_rows};
use crate::link_graph::models::{
    LinkGraphJuliaRerankTelemetry, LinkGraphRetrievalPlanRecord, QuantumContext,
};
#[cfg(feature = "julia")]
use crate::link_graph::models::{QuantumAnchorHit, QuantumSemanticSearchRequest};
use crate::link_graph::runtime_config::models::{
    LinkGraphJuliaRerankRuntimeConfig, LinkGraphSemanticIgnitionBackend,
    LinkGraphSemanticIgnitionRuntimeConfig,
};
use crate::link_graph::runtime_config::resolve_link_graph_retrieval_policy_runtime;
use crate::link_graph::{
    LinkGraphPlannedSearchPayload, LinkGraphSemanticIgnitionTelemetry,
    OpenAiCompatibleSemanticIgnition, QuantumFusionOptions, QuantumSemanticIgnition,
    VectorStoreSemanticIgnition,
};
#[cfg(feature = "julia")]
use arrow::record_batch::RecordBatch;
#[cfg(feature = "julia")]
use std::cmp::Ordering;
#[cfg(feature = "julia")]
use std::collections::BTreeMap;
use xiuxian_vector::VectorStore;
#[cfg(feature = "julia")]
use xiuxian_vector::{
    ARROW_TRANSPORT_TRACE_ID_METADATA_KEY, ArrowTransportClient, ArrowTransportConfig,
    attach_record_batch_metadata,
};

type SemanticIgnitionOutcome = Result<
    (
        Option<String>,
        Vec<QuantumContext>,
        Option<LinkGraphJuliaRerankTelemetry>,
    ),
    String,
>;

impl LinkGraphIndex {
    pub(super) async fn enrich_planned_payload_with_quantum_contexts(
        &self,
        payload: &mut LinkGraphPlannedSearchPayload,
        query_vector: &[f32],
    ) {
        let runtime = resolve_link_graph_retrieval_policy_runtime();
        let backend = runtime.semantic_ignition.backend;
        let backend_label = semantic_ignition_backend_label(backend);
        if backend_label.is_empty() {
            return;
        }

        let Some(retrieval_plan) = payload.retrieval_plan.as_ref() else {
            record_semantic_ignition_error(
                payload,
                backend_label,
                "semantic ignition skipped because retrieval plan is missing".to_string(),
            );
            return;
        };

        let (vector_store_path, table_name) =
            match resolve_vector_store_requirements(&runtime.semantic_ignition) {
                Ok(parts) => parts,
                Err(error) => {
                    record_semantic_ignition_error(payload, backend_label, error);
                    return;
                }
            };

        let store = match VectorStore::new(vector_store_path, None).await {
            Ok(store) => store,
            Err(error) => {
                record_semantic_ignition_error(
                    payload,
                    backend_label,
                    format!("failed to open vector store: {error}"),
                );
                return;
            }
        };

        let outcome = match backend {
            LinkGraphSemanticIgnitionBackend::Disabled => return,
            LinkGraphSemanticIgnitionBackend::VectorStore => {
                self.quantum_contexts_from_vector_store_runtime(
                    store,
                    table_name,
                    payload.query.as_str(),
                    query_vector,
                    retrieval_plan,
                    &runtime.julia_rerank,
                )
                .await
            }
            LinkGraphSemanticIgnitionBackend::OpenAiCompatible => {
                self.quantum_contexts_from_openai_runtime(
                    store,
                    &runtime.semantic_ignition,
                    table_name,
                    payload.query.as_str(),
                    query_vector,
                    retrieval_plan,
                    &runtime.julia_rerank,
                )
                .await
            }
        };

        apply_semantic_ignition_outcome(payload, backend_label, outcome);
    }

    async fn quantum_contexts_from_vector_store_runtime(
        &self,
        store: VectorStore,
        table_name: &str,
        query_text: &str,
        query_vector: &[f32],
        retrieval_plan: &LinkGraphRetrievalPlanRecord,
        julia_rerank: &LinkGraphJuliaRerankRuntimeConfig,
    ) -> SemanticIgnitionOutcome {
        let ignition = VectorStoreSemanticIgnition::new(store, table_name);
        let backend_name = ignition.backend_name().to_string();
        let mut contexts = self
            .quantum_contexts_from_retrieval_plan(
                &ignition,
                Some(query_text),
                query_vector,
                Some(retrieval_plan),
                None,
                &QuantumFusionOptions::default(),
            )
            .await
            .map_err(|error| error.to_string())?;
        let telemetry = apply_vector_store_julia_rerank(
            &ignition,
            julia_rerank,
            query_text,
            query_vector,
            retrieval_plan,
            &mut contexts,
        )
        .await;
        Ok((Some(backend_name), contexts, telemetry))
    }

    async fn quantum_contexts_from_openai_runtime(
        &self,
        store: VectorStore,
        config: &LinkGraphSemanticIgnitionRuntimeConfig,
        table_name: &str,
        query_text: &str,
        query_vector: &[f32],
        retrieval_plan: &LinkGraphRetrievalPlanRecord,
        julia_rerank: &LinkGraphJuliaRerankRuntimeConfig,
    ) -> SemanticIgnitionOutcome {
        let Some(embedding_base_url) = config.embedding_base_url.as_deref() else {
            return Err(
                "openai-compatible semantic ignition requires `link_graph.retrieval.semantic_ignition.embedding_base_url`"
                    .to_string(),
            );
        };
        let mut ignition =
            OpenAiCompatibleSemanticIgnition::new(store, table_name, embedding_base_url);
        if let Some(model) = config.embedding_model.as_deref() {
            ignition = ignition.with_embedding_model(model);
        }
        let backend_name = ignition.backend_name().to_string();
        let mut contexts = self
            .quantum_contexts_from_retrieval_plan(
                &ignition,
                Some(query_text),
                query_vector,
                Some(retrieval_plan),
                None,
                &QuantumFusionOptions::default(),
            )
            .await
            .map_err(|error| error.to_string())?;
        let telemetry = apply_openai_julia_rerank(
            &ignition,
            julia_rerank,
            query_text,
            retrieval_plan,
            &mut contexts,
        )
        .await;
        Ok((Some(backend_name), contexts, telemetry))
    }
}

fn resolve_vector_store_requirements(
    config: &LinkGraphSemanticIgnitionRuntimeConfig,
) -> Result<(&str, &str), String> {
    let Some(vector_store_path) = config.vector_store_path.as_deref() else {
        return Err(
            "semantic ignition requires `link_graph.retrieval.semantic_ignition.vector_store_path`"
                .to_string(),
        );
    };
    let Some(table_name) = config.table_name.as_deref() else {
        return Err(
            "semantic ignition requires `link_graph.retrieval.semantic_ignition.table_name`"
                .to_string(),
        );
    };
    Ok((vector_store_path, table_name))
}

fn apply_semantic_ignition_outcome(
    payload: &mut LinkGraphPlannedSearchPayload,
    backend_label: &str,
    outcome: SemanticIgnitionOutcome,
) {
    match outcome {
        Ok((backend_name, contexts, julia_rerank)) => {
            payload.semantic_ignition = Some(LinkGraphSemanticIgnitionTelemetry {
                backend: backend_label.to_string(),
                backend_name,
                context_count: contexts.len(),
                error: None,
            });
            payload.julia_rerank = julia_rerank;
            payload.quantum_contexts = contexts;
        }
        Err(error) => record_semantic_ignition_error(payload, backend_label, error),
    }
}

fn record_semantic_ignition_error(
    payload: &mut LinkGraphPlannedSearchPayload,
    backend_label: &str,
    error: String,
) {
    payload.semantic_ignition = Some(LinkGraphSemanticIgnitionTelemetry {
        backend: backend_label.to_string(),
        backend_name: None,
        context_count: 0,
        error: Some(error),
    });
    payload.julia_rerank = None;
    payload.quantum_contexts.clear();
}

fn semantic_ignition_backend_label(backend: LinkGraphSemanticIgnitionBackend) -> &'static str {
    match backend {
        LinkGraphSemanticIgnitionBackend::Disabled => "",
        LinkGraphSemanticIgnitionBackend::VectorStore => "vector_store",
        LinkGraphSemanticIgnitionBackend::OpenAiCompatible => "openai_compatible",
    }
}

#[cfg(feature = "julia")]
async fn apply_vector_store_julia_rerank(
    ignition: &VectorStoreSemanticIgnition,
    config: &LinkGraphJuliaRerankRuntimeConfig,
    query_text: &str,
    query_vector: &[f32],
    retrieval_plan: &LinkGraphRetrievalPlanRecord,
    contexts: &mut Vec<QuantumContext>,
) -> Option<LinkGraphJuliaRerankTelemetry> {
    if config.base_url.is_none() {
        return None;
    }

    if query_vector.is_empty() || contexts.is_empty() {
        return Some(LinkGraphJuliaRerankTelemetry {
            applied: false,
            response_row_count: 0,
            trace_ids: Vec::new(),
            error: Some(
                "link-graph Julia rerank currently requires a precomputed query vector for the vector-store semantic ignition backend".to_string(),
            ),
        });
    }

    let transport = match build_julia_rerank_transport_client(config) {
        Ok(Some(client)) => client,
        Ok(None) => return None,
        Err(error) => {
            return Some(LinkGraphJuliaRerankTelemetry {
                applied: false,
                response_row_count: 0,
                trace_ids: Vec::new(),
                error: Some(error),
            });
        }
    };
    let request = QuantumSemanticSearchRequest::from_retrieval_budget(
        Some(query_text),
        query_vector,
        Some(&retrieval_plan.budget),
        None,
    );
    let anchors = contexts
        .iter()
        .map(|context| QuantumAnchorHit {
            anchor_id: context.anchor_id.clone(),
            vector_score: context.vector_score,
        })
        .collect::<Vec<_>>();
    let request_batch = match ignition
        .build_julia_rerank_request_batch(request, &anchors)
        .await
    {
        Ok(batch) => match attach_julia_rerank_request_trace_id(batch, query_text) {
            Ok(batch) => batch,
            Err(error) => {
                return Some(LinkGraphJuliaRerankTelemetry {
                    applied: false,
                    response_row_count: 0,
                    trace_ids: Vec::new(),
                    error: Some(error),
                });
            }
        },
        Err(error) => {
            return Some(LinkGraphJuliaRerankTelemetry {
                applied: false,
                response_row_count: 0,
                trace_ids: Vec::new(),
                error: Some(format!(
                    "failed to build Julia rerank request batch: {error}"
                )),
            });
        }
    };
    let response_batches = match transport.process_batch(&request_batch).await {
        Ok(batches) => batches,
        Err(error) => {
            return Some(LinkGraphJuliaRerankTelemetry {
                applied: false,
                response_row_count: 0,
                trace_ids: Vec::new(),
                error: Some(format!("Julia rerank transport failed: {error}")),
            });
        }
    };
    if let Err(error) = validate_julia_arrow_response_batches(response_batches.as_slice()) {
        return Some(LinkGraphJuliaRerankTelemetry {
            applied: false,
            response_row_count: 0,
            trace_ids: Vec::new(),
            error: Some(format!(
                "Julia rerank response contract validation failed: {error}"
            )),
        });
    }
    let response_rows = match decode_julia_arrow_score_rows(response_batches.as_slice()) {
        Ok(rows) => rows,
        Err(error) => {
            return Some(LinkGraphJuliaRerankTelemetry {
                applied: false,
                response_row_count: 0,
                trace_ids: Vec::new(),
                error: Some(format!(
                    "failed to decode Julia rerank response rows: {error}"
                )),
            });
        }
    };
    let updated = apply_julia_rerank_scores(contexts, &response_rows);
    Some(LinkGraphJuliaRerankTelemetry {
        applied: updated > 0,
        response_row_count: response_rows.len(),
        trace_ids: collect_julia_rerank_trace_ids(&response_rows),
        error: None,
    })
}

#[cfg(not(feature = "julia"))]
async fn apply_vector_store_julia_rerank(
    _ignition: &VectorStoreSemanticIgnition,
    config: &LinkGraphJuliaRerankRuntimeConfig,
    _query_text: &str,
    _query_vector: &[f32],
    _retrieval_plan: &LinkGraphRetrievalPlanRecord,
    _contexts: &mut Vec<QuantumContext>,
) -> Option<LinkGraphJuliaRerankTelemetry> {
    config
        .base_url
        .as_ref()
        .map(|_| LinkGraphJuliaRerankTelemetry {
            applied: false,
            response_row_count: 0,
            trace_ids: Vec::new(),
            error: Some(
                "link-graph Julia rerank is configured but `xiuxian-wendao` was built without the `julia` feature".to_string(),
            ),
        })
}

#[cfg(feature = "julia")]
async fn apply_openai_julia_rerank(
    ignition: &OpenAiCompatibleSemanticIgnition,
    config: &LinkGraphJuliaRerankRuntimeConfig,
    query_text: &str,
    retrieval_plan: &LinkGraphRetrievalPlanRecord,
    contexts: &mut Vec<QuantumContext>,
) -> Option<LinkGraphJuliaRerankTelemetry> {
    if config.base_url.is_none() || contexts.is_empty() {
        return None;
    }

    let transport = match build_julia_rerank_transport_client(config) {
        Ok(Some(client)) => client,
        Ok(None) => return None,
        Err(error) => {
            return Some(LinkGraphJuliaRerankTelemetry {
                applied: false,
                response_row_count: 0,
                trace_ids: Vec::new(),
                error: Some(error),
            });
        }
    };

    let request = QuantumSemanticSearchRequest::from_retrieval_budget(
        Some(query_text),
        &[],
        Some(&retrieval_plan.budget),
        None,
    );
    let anchors = contexts
        .iter()
        .map(|context| QuantumAnchorHit {
            anchor_id: context.anchor_id.clone(),
            vector_score: context.vector_score,
        })
        .collect::<Vec<_>>();
    let request_batch = match ignition
        .build_julia_rerank_request_batch(request, &anchors)
        .await
    {
        Ok(batch) => match attach_julia_rerank_request_trace_id(batch, query_text) {
            Ok(batch) => batch,
            Err(error) => {
                return Some(LinkGraphJuliaRerankTelemetry {
                    applied: false,
                    response_row_count: 0,
                    trace_ids: Vec::new(),
                    error: Some(error),
                });
            }
        },
        Err(error) => {
            return Some(LinkGraphJuliaRerankTelemetry {
                applied: false,
                response_row_count: 0,
                trace_ids: Vec::new(),
                error: Some(format!(
                    "failed to build Julia rerank request batch: {error}"
                )),
            });
        }
    };

    let response_batches = match transport.process_batch(&request_batch).await {
        Ok(batches) => batches,
        Err(error) => {
            return Some(LinkGraphJuliaRerankTelemetry {
                applied: false,
                response_row_count: 0,
                trace_ids: Vec::new(),
                error: Some(format!("Julia rerank transport failed: {error}")),
            });
        }
    };
    if let Err(error) = validate_julia_arrow_response_batches(response_batches.as_slice()) {
        return Some(LinkGraphJuliaRerankTelemetry {
            applied: false,
            response_row_count: 0,
            trace_ids: Vec::new(),
            error: Some(format!(
                "Julia rerank response contract validation failed: {error}"
            )),
        });
    }

    let response_rows = match decode_julia_arrow_score_rows(response_batches.as_slice()) {
        Ok(rows) => rows,
        Err(error) => {
            return Some(LinkGraphJuliaRerankTelemetry {
                applied: false,
                response_row_count: 0,
                trace_ids: Vec::new(),
                error: Some(format!(
                    "failed to decode Julia rerank response rows: {error}"
                )),
            });
        }
    };

    let updated = apply_julia_rerank_scores(contexts, &response_rows);
    Some(LinkGraphJuliaRerankTelemetry {
        applied: updated > 0,
        response_row_count: response_rows.len(),
        trace_ids: collect_julia_rerank_trace_ids(&response_rows),
        error: None,
    })
}

#[cfg(not(feature = "julia"))]
async fn apply_openai_julia_rerank(
    _ignition: &OpenAiCompatibleSemanticIgnition,
    config: &LinkGraphJuliaRerankRuntimeConfig,
    _query_text: &str,
    _retrieval_plan: &LinkGraphRetrievalPlanRecord,
    _contexts: &mut Vec<QuantumContext>,
) -> Option<LinkGraphJuliaRerankTelemetry> {
    config
        .base_url
        .as_ref()
        .map(|_| LinkGraphJuliaRerankTelemetry {
            applied: false,
            response_row_count: 0,
            trace_ids: Vec::new(),
            error: Some(
                "link-graph Julia rerank is configured but `xiuxian-wendao` was built without the `julia` feature".to_string(),
            ),
        })
}

#[cfg(feature = "julia")]
fn build_julia_rerank_transport_client(
    config: &LinkGraphJuliaRerankRuntimeConfig,
) -> Result<Option<ArrowTransportClient>, String> {
    let Some(base_url) = config.base_url.as_deref() else {
        return Ok(None);
    };

    let mut resolved = ArrowTransportConfig::new(base_url);
    if let Some(route) = config.route.as_deref() {
        resolved = resolved.with_route(route);
    }
    if let Some(health_route) = config.health_route.as_deref() {
        resolved = resolved.with_health_route(health_route);
    }
    if let Some(schema_version) = config.schema_version.as_deref() {
        resolved = resolved
            .with_schema_version(schema_version)
            .map_err(|error| format!("invalid Julia rerank schema version: {error}"))?;
    }
    if let Some(timeout_secs) = config.timeout_secs {
        resolved = resolved
            .with_timeout_secs(timeout_secs)
            .map_err(|error| format!("invalid Julia rerank timeout: {error}"))?;
    }
    ArrowTransportClient::new(resolved)
        .map(Some)
        .map_err(|error| format!("failed to construct Julia rerank transport client: {error}"))
}

#[cfg(feature = "julia")]
fn apply_julia_rerank_scores(
    contexts: &mut [QuantumContext],
    response_rows: &BTreeMap<String, JuliaArrowScoreRow>,
) -> usize {
    let mut updated = 0usize;
    for context in contexts.iter_mut() {
        let Some(score_row) = response_rows.get(context.anchor_id.as_str()) else {
            continue;
        };
        context.saliency_score = score_row.final_score;
        updated += 1;
    }
    contexts.sort_by(|left, right| {
        right
            .saliency_score
            .partial_cmp(&left.saliency_score)
            .unwrap_or(Ordering::Equal)
            .then(left.anchor_id.cmp(&right.anchor_id))
    });
    updated
}

#[cfg(feature = "julia")]
fn collect_julia_rerank_trace_ids(
    response_rows: &BTreeMap<String, JuliaArrowScoreRow>,
) -> Vec<String> {
    response_rows
        .values()
        .filter_map(|row| row.trace_id.as_ref())
        .filter(|trace_id| !trace_id.trim().is_empty())
        .cloned()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(feature = "julia")]
fn attach_julia_rerank_request_trace_id(
    batch: RecordBatch,
    query_text: &str,
) -> Result<RecordBatch, String> {
    attach_record_batch_metadata(
        &batch,
        [(
            ARROW_TRANSPORT_TRACE_ID_METADATA_KEY,
            julia_rerank_request_trace_id(query_text),
        )],
    )
    .map_err(|error| format!("failed to attach Julia rerank trace metadata: {error}"))
}

#[cfg(feature = "julia")]
fn julia_rerank_request_trace_id(query_text: &str) -> String {
    let normalized = query_text
        .split_whitespace()
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    if normalized.is_empty() {
        "julia-rerank:query".to_string()
    } else {
        format!("julia-rerank:{normalized}")
    }
}

#[cfg(all(test, feature = "julia"))]
mod tests {
    use super::*;
    use arrow::array::StringArray;
    use arrow::datatypes::{DataType, Field, Schema};
    use std::sync::Arc;

    #[test]
    fn apply_julia_rerank_scores_overwrites_saliency_and_resorts_contexts() {
        let mut contexts = vec![
            QuantumContext {
                anchor_id: "doc-1#a".to_string(),
                doc_id: "doc-1".to_string(),
                path: "notes/doc-1.md".to_string(),
                semantic_path: vec![],
                trace_label: None,
                related_clusters: vec![],
                saliency_score: 0.2,
                vector_score: 0.6,
                topology_score: 0.1,
            },
            QuantumContext {
                anchor_id: "doc-2#b".to_string(),
                doc_id: "doc-2".to_string(),
                path: "notes/doc-2.md".to_string(),
                semantic_path: vec![],
                trace_label: None,
                related_clusters: vec![],
                saliency_score: 0.9,
                vector_score: 0.5,
                topology_score: 0.2,
            },
        ];
        let response_rows = BTreeMap::from([
            (
                "doc-1#a".to_string(),
                JuliaArrowScoreRow {
                    doc_id: "doc-1#a".to_string(),
                    analyzer_score: 0.7,
                    final_score: 0.95,
                    trace_id: None,
                },
            ),
            (
                "doc-2#b".to_string(),
                JuliaArrowScoreRow {
                    doc_id: "doc-2#b".to_string(),
                    analyzer_score: 0.3,
                    final_score: 0.4,
                    trace_id: None,
                },
            ),
        ]);

        let updated = apply_julia_rerank_scores(&mut contexts, &response_rows);

        assert_eq!(updated, 2);
        assert_eq!(contexts[0].anchor_id, "doc-1#a");
        assert!((contexts[0].saliency_score - 0.95).abs() < f64::EPSILON);
        assert_eq!(contexts[1].anchor_id, "doc-2#b");
        assert!((contexts[1].saliency_score - 0.4).abs() < f64::EPSILON);
    }

    #[test]
    fn build_julia_rerank_transport_client_honors_runtime_overrides() {
        let client = build_julia_rerank_transport_client(&LinkGraphJuliaRerankRuntimeConfig {
            base_url: Some("http://127.0.0.1:8090".to_string()),
            route: Some("/custom-ipc".to_string()),
            health_route: Some("/healthz".to_string()),
            schema_version: Some("v1".to_string()),
            timeout_secs: Some(15),
            service_mode: None,
            analyzer_config_path: None,
            analyzer_strategy: None,
            vector_weight: None,
            similarity_weight: None,
        })
        .expect("config should be valid")
        .expect("base url should enable transport");

        assert_eq!(client.config().base_url(), "http://127.0.0.1:8090");
        assert_eq!(client.config().route(), "/custom-ipc");
        assert_eq!(client.config().health_route(), "/healthz");
        assert_eq!(client.config().schema_version(), "v1");
        assert_eq!(client.config().timeout().as_secs(), 15);
    }

    #[test]
    fn collect_julia_rerank_trace_ids_deduplicates_non_empty_values() {
        let response_rows = BTreeMap::from([
            (
                "doc-1#a".to_string(),
                JuliaArrowScoreRow {
                    doc_id: "doc-1#a".to_string(),
                    analyzer_score: 0.7,
                    final_score: 0.95,
                    trace_id: Some("trace-123".to_string()),
                },
            ),
            (
                "doc-2#b".to_string(),
                JuliaArrowScoreRow {
                    doc_id: "doc-2#b".to_string(),
                    analyzer_score: 0.3,
                    final_score: 0.4,
                    trace_id: Some("trace-123".to_string()),
                },
            ),
            (
                "doc-3#c".to_string(),
                JuliaArrowScoreRow {
                    doc_id: "doc-3#c".to_string(),
                    analyzer_score: 0.2,
                    final_score: 0.1,
                    trace_id: Some("trace-456".to_string()),
                },
            ),
        ]);

        let trace_ids = collect_julia_rerank_trace_ids(&response_rows);

        assert_eq!(
            trace_ids,
            vec!["trace-123".to_string(), "trace-456".to_string()]
        );
    }

    #[test]
    fn julia_rerank_request_trace_id_normalizes_query_text() {
        assert_eq!(
            julia_rerank_request_trace_id("  alpha   signal "),
            "julia-rerank:alpha_signal"
        );
        assert_eq!(julia_rerank_request_trace_id(""), "julia-rerank:query");
    }

    #[test]
    fn attach_julia_rerank_request_trace_id_sets_schema_metadata() {
        let batch = RecordBatch::try_new(
            Arc::new(Schema::new(vec![Field::new(
                "doc_id",
                DataType::Utf8,
                false,
            )])),
            vec![Arc::new(StringArray::from(vec!["doc-1"]))],
        )
        .expect("batch");

        let traced_batch =
            attach_julia_rerank_request_trace_id(batch, "alpha signal").expect("metadata");

        assert_eq!(
            traced_batch.schema().metadata().get("trace_id"),
            Some(&"julia-rerank:alpha_signal".to_string())
        );
    }
}
