use axum::Extension;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde_json::Value;
use std::sync::Arc;
use std::time::Duration;
use xiuxian_memory_engine::IntentEncoder;

use super::runtime::resolve_embed_model;
use super::types::{
    EmbedBatchRequest, EmbedBatchResponse, EmbedRequest, EmbedResponse, GatewayEmbeddingRuntime,
    GatewayHealthResponse, GatewayJsonError, GatewayJsonResult, GatewayMcpHealthResponse,
    GatewayState, MessageRequest, MessageResponse, OpenAiEmbeddingData, OpenAiEmbeddingUsage,
    OpenAiEmbeddingsRequest, OpenAiEmbeddingsResponse,
};

/// Validate request body; returns error for empty `session_id` or message.
///
/// # Errors
/// Returns an HTTP 400 tuple when request fields are empty after trimming.
pub fn validate_message_request(
    body: &MessageRequest,
) -> Result<(String, String), (StatusCode, String)> {
    let session_id = body.session_id.trim().to_string();
    let message = body.message.trim().to_string();
    if session_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "session_id must be non-empty".to_string(),
        ));
    }
    if message.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "message must be non-empty".to_string(),
        ));
    }
    Ok((session_id, message))
}

pub(super) async fn handle_message(
    State(state): State<GatewayState>,
    Json(body): Json<MessageRequest>,
) -> GatewayJsonResult<MessageResponse> {
    let (session_id, message) = validate_message_request(&body)?;
    let _permit = if let Some(ref sem) = state.concurrency_semaphore {
        Some(sem.acquire().await.map_err(|_| {
            (
                StatusCode::SERVICE_UNAVAILABLE,
                "concurrency limit closed".to_string(),
            )
        })?)
    } else {
        None
    };

    let timeout_secs = state.turn_timeout_secs;
    let output = match tokio::time::timeout(
        Duration::from_secs(timeout_secs),
        state.agent.run_turn(&session_id, &message),
    )
    .await
    {
        Ok(Ok(out)) => out,
        Ok(Err(error)) => return Err((StatusCode::INTERNAL_SERVER_ERROR, error.to_string())),
        Err(_) => {
            return Err((
                StatusCode::GATEWAY_TIMEOUT,
                format!("agent turn timed out after {timeout_secs}s"),
            ));
        }
    };

    Ok(Json(MessageResponse { output, session_id }))
}

fn parse_openai_embedding_input(input: &Value) -> Result<Vec<String>, GatewayJsonError> {
    match input {
        Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                Err((
                    StatusCode::BAD_REQUEST,
                    "input text must be non-empty".to_string(),
                ))
            } else {
                Ok(vec![trimmed.to_string()])
            }
        }
        Value::Array(items) => {
            if items.is_empty() {
                return Err((
                    StatusCode::BAD_REQUEST,
                    "input array must be non-empty".to_string(),
                ));
            }

            let mut texts = Vec::with_capacity(items.len());
            for item in items {
                let Some(text) = item.as_str() else {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "input array must contain non-empty strings".to_string(),
                    ));
                };
                let trimmed = text.trim();
                if trimmed.is_empty() {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        "input array must contain non-empty strings".to_string(),
                    ));
                }
                texts.push(trimmed.to_string());
            }
            Ok(texts)
        }
        _ => Err((
            StatusCode::BAD_REQUEST,
            "input must be a string or an array of strings".to_string(),
        )),
    }
}

fn fallback_hash_embed_single(text: &str, dimension: usize) -> Vec<f32> {
    let encoder = IntentEncoder::new(dimension);
    encoder.encode(text)
}

pub(super) fn fallback_hash_embed_batch(texts: &[String], dimension: usize) -> Vec<Vec<f32>> {
    let encoder = IntentEncoder::new(dimension);
    texts.iter().map(|text| encoder.encode(text)).collect()
}

async fn guarded_embed_batch(
    embedding_runtime: Arc<GatewayEmbeddingRuntime>,
    texts: &[String],
    model: &str,
) -> Option<Vec<Vec<f32>>> {
    let model = model.to_string();
    let texts = texts.to_vec();
    let client = Arc::clone(&embedding_runtime.client);
    match tokio::spawn(async move {
        client
            .embed_batch_with_model(&texts, Some(model.as_str()))
            .await
    })
    .await
    {
        Ok(vectors) => vectors,
        Err(error) => {
            tracing::warn!(
                event = "gateway.embedding.task_panicked",
                endpoint = "/embed/batch",
                error = %error,
                "embedding task panicked; using deterministic hash fallback"
            );
            None
        }
    }
}

async fn guarded_embed_single(
    embedding_runtime: Arc<GatewayEmbeddingRuntime>,
    text: &str,
    model: &str,
) -> Option<Vec<f32>> {
    let model = model.to_string();
    let text = text.to_string();
    let client = Arc::clone(&embedding_runtime.client);
    match tokio::spawn(async move {
        client
            .embed_with_model(text.as_str(), Some(model.as_str()))
            .await
    })
    .await
    {
        Ok(vector) => vector,
        Err(error) => {
            tracing::warn!(
                event = "gateway.embedding.task_panicked",
                endpoint = "/embed/single",
                error = %error,
                "embedding task panicked; using deterministic hash fallback"
            );
            None
        }
    }
}

pub(super) async fn handle_embed_batch(
    Extension(embedding_runtime): Extension<Arc<GatewayEmbeddingRuntime>>,
    Json(body): Json<EmbedBatchRequest>,
) -> GatewayJsonResult<EmbedBatchResponse> {
    if body.texts.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "texts must be a non-empty array".to_string(),
        ));
    }

    let model = resolve_embed_model(
        body.model.as_deref(),
        embedding_runtime.default_model.as_deref(),
    )?;

    let vectors = if let Some(vectors) =
        guarded_embed_batch(Arc::clone(&embedding_runtime), &body.texts, model.as_str()).await
    {
        vectors
    } else {
        tracing::warn!(
            event = "gateway.embedding.fallback.hash_encoder",
            endpoint = "/embed/batch",
            model,
            fallback_dim = embedding_runtime.fallback_embedding_dim,
            "embedding backend unavailable; serving deterministic hash embedding fallback"
        );
        fallback_hash_embed_batch(&body.texts, embedding_runtime.fallback_embedding_dim)
    };

    if vectors.len() != body.texts.len() {
        return Err((
            StatusCode::BAD_GATEWAY,
            "embedding backend returned invalid vector count".to_string(),
        ));
    }

    Ok(Json(EmbedBatchResponse { vectors }))
}

pub(super) async fn handle_embed(
    Extension(embedding_runtime): Extension<Arc<GatewayEmbeddingRuntime>>,
    Json(body): Json<EmbedRequest>,
) -> GatewayJsonResult<EmbedResponse> {
    let text = body.text.trim();
    if text.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "text must be non-empty".to_string(),
        ));
    }

    let model = resolve_embed_model(
        body.model.as_deref(),
        embedding_runtime.default_model.as_deref(),
    )?;

    let vector = if let Some(vector) =
        guarded_embed_single(Arc::clone(&embedding_runtime), text, model.as_str()).await
    {
        vector
    } else {
        tracing::warn!(
            event = "gateway.embedding.fallback.hash_encoder",
            endpoint = "/embed/single",
            model,
            fallback_dim = embedding_runtime.fallback_embedding_dim,
            "embedding backend unavailable; serving deterministic hash embedding fallback"
        );
        fallback_hash_embed_single(text, embedding_runtime.fallback_embedding_dim)
    };

    Ok(Json(EmbedResponse { vector }))
}

pub(super) async fn handle_openai_embeddings(
    Extension(embedding_runtime): Extension<Arc<GatewayEmbeddingRuntime>>,
    Json(body): Json<OpenAiEmbeddingsRequest>,
) -> GatewayJsonResult<OpenAiEmbeddingsResponse> {
    let texts = parse_openai_embedding_input(&body.input)?;
    let model = resolve_embed_model(
        body.model.as_deref(),
        embedding_runtime.default_model.as_deref(),
    )?;

    let vectors = if let Some(vectors) =
        guarded_embed_batch(Arc::clone(&embedding_runtime), &texts, model.as_str()).await
    {
        vectors
    } else {
        tracing::warn!(
            event = "gateway.embedding.fallback.hash_encoder",
            endpoint = "/v1/embeddings",
            model,
            fallback_dim = embedding_runtime.fallback_embedding_dim,
            "embedding backend unavailable; serving deterministic hash embedding fallback"
        );
        fallback_hash_embed_batch(&texts, embedding_runtime.fallback_embedding_dim)
    };

    if vectors.len() != texts.len() {
        return Err((
            StatusCode::BAD_GATEWAY,
            "embedding backend returned invalid vector count".to_string(),
        ));
    }

    let data = vectors
        .into_iter()
        .enumerate()
        .map(|(index, embedding)| OpenAiEmbeddingData {
            object: "embedding",
            index,
            embedding,
        })
        .collect::<Vec<_>>();

    Ok(Json(OpenAiEmbeddingsResponse {
        object: "list",
        model,
        data,
        usage: OpenAiEmbeddingUsage {
            prompt_tokens: 0,
            total_tokens: 0,
        },
    }))
}

pub(super) async fn handle_health(
    State(state): State<GatewayState>,
) -> Json<GatewayHealthResponse> {
    let mcp_cache = state.agent.inspect_mcp_tools_list_cache_stats();
    let in_flight_turns = state.max_concurrent_turns.and_then(|max| {
        state
            .concurrency_semaphore
            .as_ref()
            .map(|sem| max.saturating_sub(sem.available_permits()))
    });

    Json(GatewayHealthResponse {
        status: "healthy",
        turn_timeout_secs: state.turn_timeout_secs,
        max_concurrent_turns: state.max_concurrent_turns,
        in_flight_turns,
        mcp: GatewayMcpHealthResponse {
            enabled: mcp_cache.is_some(),
            tools_list_cache: mcp_cache,
        },
    })
}
