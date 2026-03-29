#[cfg(feature = "julia")]
use crate::analyzers::languages::validate_julia_arrow_response_batches;
#[cfg(feature = "julia")]
use crate::analyzers::{PluginArrowScoreRow, decode_plugin_arrow_score_rows};
use crate::link_graph::models::{
    LinkGraphJuliaRerankTelemetry, LinkGraphRetrievalPlanRecord, QuantumContext,
};
#[cfg(feature = "julia")]
use crate::link_graph::models::{QuantumAnchorHit, QuantumSemanticSearchRequest};
#[cfg(feature = "julia")]
use crate::link_graph::plugin_runtime::build_arrow_transport_client_from_binding;
use crate::link_graph::{OpenAiCompatibleSemanticIgnition, VectorStoreSemanticIgnition};
#[cfg(feature = "julia")]
use arrow::record_batch::RecordBatch;
#[cfg(feature = "julia")]
use std::cmp::Ordering;
#[cfg(feature = "julia")]
use std::collections::BTreeMap;
#[cfg(feature = "julia")]
use xiuxian_vector::{
    ARROW_TRANSPORT_TRACE_ID_METADATA_KEY, ArrowTransportClient, attach_record_batch_metadata,
};
use xiuxian_wendao_core::capabilities::PluginCapabilityBinding;

#[cfg(feature = "julia")]
pub(super) async fn apply_vector_store_plugin_rerank(
    ignition: &VectorStoreSemanticIgnition,
    binding: Option<&PluginCapabilityBinding>,
    query_text: &str,
    query_vector: &[f32],
    retrieval_plan: &LinkGraphRetrievalPlanRecord,
    contexts: &mut Vec<QuantumContext>,
) -> Option<LinkGraphJuliaRerankTelemetry> {
    let Some(binding) = binding else {
        return None;
    };

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

    let transport = match build_plugin_rerank_transport_client(binding) {
        Ok(Some(client)) => client,
        Ok(None) => return None,
        Err(error) => return Some(plugin_rerank_error_telemetry(error)),
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
    let request_batch = match build_vector_store_plugin_rerank_request_batch(
        ignition,
        request,
        &anchors,
        query_text,
    )
    .await
    {
        Ok(batch) => batch,
        Err(error) => return Some(plugin_rerank_error_telemetry(error)),
    };
    let response_batches = match transport.process_batch(&request_batch).await {
        Ok(batches) => batches,
        Err(error) => {
            return Some(plugin_rerank_error_telemetry(format!(
                "Julia rerank transport failed: {error}"
            )));
        }
    };
    let response_rows = match decode_plugin_rerank_response_rows(response_batches.as_slice()) {
        Ok(rows) => rows,
        Err(error) => return Some(plugin_rerank_error_telemetry(error)),
    };
    let updated = apply_plugin_rerank_scores(contexts, &response_rows);
    Some(LinkGraphJuliaRerankTelemetry {
        applied: updated > 0,
        response_row_count: response_rows.len(),
        trace_ids: collect_plugin_rerank_trace_ids(&response_rows),
        error: None,
    })
}

#[cfg(not(feature = "julia"))]
pub(super) async fn apply_vector_store_plugin_rerank(
    _ignition: &VectorStoreSemanticIgnition,
    binding: Option<&PluginCapabilityBinding>,
    _query_text: &str,
    _query_vector: &[f32],
    _retrieval_plan: &LinkGraphRetrievalPlanRecord,
    _contexts: &mut Vec<QuantumContext>,
) -> Option<LinkGraphJuliaRerankTelemetry> {
    binding
        .and_then(|binding| binding.endpoint.base_url.as_ref())
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
pub(super) async fn apply_openai_plugin_rerank(
    ignition: &OpenAiCompatibleSemanticIgnition,
    binding: Option<&PluginCapabilityBinding>,
    query_text: &str,
    retrieval_plan: &LinkGraphRetrievalPlanRecord,
    contexts: &mut Vec<QuantumContext>,
) -> Option<LinkGraphJuliaRerankTelemetry> {
    let Some(binding) = binding else {
        return None;
    };
    if binding.endpoint.base_url.is_none() || contexts.is_empty() {
        return None;
    }

    let transport = match build_plugin_rerank_transport_client(binding) {
        Ok(Some(client)) => client,
        Ok(None) => return None,
        Err(error) => return Some(plugin_rerank_error_telemetry(error)),
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
    let request_batch = match build_openai_plugin_rerank_request_batch(
        ignition,
        request,
        &anchors,
        query_text,
    )
    .await
    {
        Ok(batch) => batch,
        Err(error) => return Some(plugin_rerank_error_telemetry(error)),
    };
    let response_batches = match transport.process_batch(&request_batch).await {
        Ok(batches) => batches,
        Err(error) => {
            return Some(plugin_rerank_error_telemetry(format!(
                "Julia rerank transport failed: {error}"
            )));
        }
    };
    let response_rows = match decode_plugin_rerank_response_rows(response_batches.as_slice()) {
        Ok(rows) => rows,
        Err(error) => return Some(plugin_rerank_error_telemetry(error)),
    };

    let updated = apply_plugin_rerank_scores(contexts, &response_rows);
    Some(LinkGraphJuliaRerankTelemetry {
        applied: updated > 0,
        response_row_count: response_rows.len(),
        trace_ids: collect_plugin_rerank_trace_ids(&response_rows),
        error: None,
    })
}

#[cfg(not(feature = "julia"))]
pub(super) async fn apply_openai_plugin_rerank(
    _ignition: &OpenAiCompatibleSemanticIgnition,
    binding: Option<&PluginCapabilityBinding>,
    _query_text: &str,
    _retrieval_plan: &LinkGraphRetrievalPlanRecord,
    _contexts: &mut Vec<QuantumContext>,
) -> Option<LinkGraphJuliaRerankTelemetry> {
    binding
        .and_then(|binding| binding.endpoint.base_url.as_ref())
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
pub(super) fn build_plugin_rerank_transport_client(
    binding: &PluginCapabilityBinding,
) -> Result<Option<ArrowTransportClient>, String> {
    build_arrow_transport_client_from_binding(binding)
}

#[cfg(feature = "julia")]
async fn build_vector_store_plugin_rerank_request_batch(
    ignition: &VectorStoreSemanticIgnition,
    request: QuantumSemanticSearchRequest<'_>,
    anchors: &[QuantumAnchorHit],
    query_text: &str,
) -> Result<RecordBatch, String> {
    let batch = ignition
        .build_plugin_rerank_request_batch(request, anchors)
        .await
        .map_err(|error| format!("failed to build Julia rerank request batch: {error}"))?;
    attach_plugin_rerank_request_trace_id(batch, query_text)
}

#[cfg(feature = "julia")]
async fn build_openai_plugin_rerank_request_batch(
    ignition: &OpenAiCompatibleSemanticIgnition,
    request: QuantumSemanticSearchRequest<'_>,
    anchors: &[QuantumAnchorHit],
    query_text: &str,
) -> Result<RecordBatch, String> {
    let batch = ignition
        .build_plugin_rerank_request_batch(request, anchors)
        .await
        .map_err(|error| format!("failed to build Julia rerank request batch: {error}"))?;
    attach_plugin_rerank_request_trace_id(batch, query_text)
}

#[cfg(feature = "julia")]
fn decode_plugin_rerank_response_rows(
    response_batches: &[RecordBatch],
) -> Result<BTreeMap<String, PluginArrowScoreRow>, String> {
    validate_julia_arrow_response_batches(response_batches)
        .map_err(|error| format!("Julia rerank response contract validation failed: {error}"))?;
    decode_plugin_arrow_score_rows(response_batches)
        .map_err(|error| format!("failed to decode Julia rerank response rows: {error}"))
}

#[cfg(feature = "julia")]
fn plugin_rerank_error_telemetry(error: String) -> LinkGraphJuliaRerankTelemetry {
    LinkGraphJuliaRerankTelemetry {
        applied: false,
        response_row_count: 0,
        trace_ids: Vec::new(),
        error: Some(error),
    }
}

#[cfg(feature = "julia")]
pub(super) fn apply_plugin_rerank_scores(
    contexts: &mut [QuantumContext],
    response_rows: &BTreeMap<String, PluginArrowScoreRow>,
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
pub(super) fn collect_plugin_rerank_trace_ids(
    response_rows: &BTreeMap<String, PluginArrowScoreRow>,
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
pub(super) fn attach_plugin_rerank_request_trace_id(
    batch: RecordBatch,
    query_text: &str,
) -> Result<RecordBatch, String> {
    attach_record_batch_metadata(
        &batch,
        [(
            ARROW_TRANSPORT_TRACE_ID_METADATA_KEY,
            plugin_rerank_request_trace_id(query_text),
        )],
    )
    .map_err(|error| format!("failed to attach Julia rerank trace metadata: {error}"))
}

#[cfg(feature = "julia")]
pub(super) fn plugin_rerank_request_trace_id(query_text: &str) -> String {
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
