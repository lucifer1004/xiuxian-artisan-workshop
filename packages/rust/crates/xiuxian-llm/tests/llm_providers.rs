//! Integration tests for `xiuxian_llm::llm::providers`.

use serde_json::json;

#[cfg(feature = "provider-litellm")]
use xiuxian_llm::llm::providers::execute_anthropic_messages_from_litellm_request_with_image_hook;
use xiuxian_llm::llm::providers::{
    AnthropicCustomBaseTransport, anthropic_custom_base_transport_label,
    anthropic_custom_base_transport_order, anthropic_messages_endpoint_from_base,
    execute_anthropic_custom_base_fallback, is_anthropic_protocol_mismatch,
    is_official_anthropic_base, normalize_anthropic_image_media_type,
    normalize_optional_base_override, parse_anthropic_messages_response, parse_positive_usize,
    resolve_api_key_with_env, resolve_custom_base_transport_api_key_from_values,
    resolve_positive_usize_env, resolve_required_api_key_with_env,
    should_bypass_anthropic_model_validation, should_use_openai_like_for_base,
    summarize_anthropic_custom_base_failures,
};
#[cfg(feature = "provider-litellm")]
use xiuxian_llm::llm::providers::{
    build_openai_like_provider, build_openai_provider, inline_openai_compatible_image_urls,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn anthropic_endpoint_builder_handles_common_shapes() {
    assert_eq!(
        anthropic_messages_endpoint_from_base("https://proxy.example.com/api"),
        "https://proxy.example.com/api/v1/messages"
    );
    assert_eq!(
        anthropic_messages_endpoint_from_base("https://proxy.example.com/api/v1"),
        "https://proxy.example.com/api/v1/messages"
    );
    assert_eq!(
        anthropic_messages_endpoint_from_base("https://proxy.example.com/api/v1/messages"),
        "https://proxy.example.com/api/v1/messages"
    );
}

#[test]
fn official_anthropic_base_is_not_bypassed() {
    assert!(is_official_anthropic_base("https://api.anthropic.com"));
    assert!(is_official_anthropic_base("https://api.anthropic.com/v1"));
    assert!(!should_bypass_anthropic_model_validation(
        "https://api.anthropic.com/v1"
    ));
}

#[test]
fn custom_anthropic_base_is_bypassed() {
    assert!(!is_official_anthropic_base("https://proxy.example.com/api"));
    assert!(should_bypass_anthropic_model_validation(
        "https://proxy.example.com/api"
    ));
}

#[test]
fn openai_base_selection_uses_official_transport_for_openai_host_only() {
    assert!(!should_use_openai_like_for_base(
        "https://api.openai.com/v1"
    ));
    assert!(should_use_openai_like_for_base(
        "https://proxy.example.com/openai"
    ));
    assert!(should_use_openai_like_for_base("http://127.0.0.1:4000/v1"));
    assert!(should_use_openai_like_for_base("not-a-url"));
}

#[test]
fn parse_anthropic_messages_response_extracts_text_and_tool_use() -> TestResult {
    let payload = json!({
        "content": [
            { "type": "text", "text": "hello " },
            { "type": "text", "text": "world" },
            { "type": "tool_use", "id": "call_1", "name": "search", "input": { "q": "rust" } }
        ]
    });

    let parsed = parse_anthropic_messages_response(&payload)?;
    assert_eq!(parsed.text.as_deref(), Some("hello world"));
    assert_eq!(parsed.tool_uses.len(), 1);
    assert_eq!(parsed.tool_uses[0].id, "call_1");
    assert_eq!(parsed.tool_uses[0].name, "search");
    assert_eq!(parsed.tool_uses[0].input, json!({ "q": "rust" }));
    Ok(())
}

#[test]
fn normalize_anthropic_image_media_type_accepts_supported_types() {
    assert_eq!(
        normalize_anthropic_image_media_type("image/png", "iVBORw0KGgo="),
        "image/png"
    );
    assert_eq!(
        normalize_anthropic_image_media_type("image/jpg", "/9j/2w=="),
        "image/jpeg"
    );
}

#[test]
fn normalize_anthropic_image_media_type_recovers_from_octet_stream() {
    assert_eq!(
        normalize_anthropic_image_media_type("application/octet-stream", "/9j/2w=="),
        "image/jpeg"
    );
    assert_eq!(
        normalize_anthropic_image_media_type("application/octet-stream", "iVBORw0KGgo="),
        "image/png"
    );
}

#[test]
fn normalize_anthropic_image_media_type_recovers_from_data_url_payload() {
    assert_eq!(
        normalize_anthropic_image_media_type(
            "application/octet-stream",
            "data:image/png;base64,iVBORw0KGgo="
        ),
        "image/png"
    );
    assert_eq!(
        normalize_anthropic_image_media_type(
            "application/octet-stream",
            "data:image/jpeg;base64,/9j/2w=="
        ),
        "image/jpeg"
    );
}

#[test]
fn anthropic_protocol_mismatch_detector_matches_400_message_errors() {
    assert!(is_anthropic_protocol_mismatch(
        "litellm-rs anthropic chat completion failed (custom-base bypass): HTTP 400 Bad Request: {\"error\":{\"message\":\"messages 参数非法。请检查文档。\"}}"
    ));
    assert!(!is_anthropic_protocol_mismatch(
        "litellm-rs anthropic chat completion failed (custom-base bypass): HTTP 429 Too Many Requests"
    ));
}

#[test]
fn custom_base_transport_order_prefers_minimax_for_glm_family() {
    assert_eq!(
        anthropic_custom_base_transport_order("glm-5"),
        [
            AnthropicCustomBaseTransport::Minimax,
            AnthropicCustomBaseTransport::OpenAi,
            AnthropicCustomBaseTransport::AnthropicMessagesBypass
        ]
    );
    assert_eq!(
        anthropic_custom_base_transport_order("claude-3-5-sonnet-20241022"),
        [
            AnthropicCustomBaseTransport::OpenAi,
            AnthropicCustomBaseTransport::Minimax,
            AnthropicCustomBaseTransport::AnthropicMessagesBypass
        ]
    );
}

#[test]
fn custom_base_transport_api_key_precedence_is_stable() {
    let openai = resolve_custom_base_transport_api_key_from_values(
        AnthropicCustomBaseTransport::OpenAi,
        None,
        Some("configured"),
        Some("openai"),
        Some("minimax"),
        Some("anthropic"),
    );
    assert_eq!(openai.as_deref(), Some("openai"));

    let bypass = resolve_custom_base_transport_api_key_from_values(
        AnthropicCustomBaseTransport::AnthropicMessagesBypass,
        None,
        Some("configured"),
        Some("openai"),
        Some("minimax"),
        Some("anthropic"),
    );
    assert_eq!(bypass.as_deref(), Some("configured"));

    let explicit = resolve_custom_base_transport_api_key_from_values(
        AnthropicCustomBaseTransport::Minimax,
        Some("explicit"),
        Some("configured"),
        Some("openai"),
        Some("minimax"),
        Some("anthropic"),
    );
    assert_eq!(explicit.as_deref(), Some("explicit"));
}

#[test]
fn summarize_custom_base_failures_renders_transport_labels() {
    let attempts = vec![
        (AnthropicCustomBaseTransport::OpenAi, "openai failed"),
        (AnthropicCustomBaseTransport::Minimax, "minimax failed"),
        (
            AnthropicCustomBaseTransport::AnthropicMessagesBypass,
            "bypass failed",
        ),
    ];
    let summary = summarize_anthropic_custom_base_failures(&attempts);
    assert_eq!(
        summary,
        "openai: openai failed | minimax: minimax failed | anthropic_messages_bypass: bypass failed"
    );
}

#[tokio::test]
async fn execute_custom_base_fallback_stops_on_first_success() -> TestResult {
    let mut seen = Vec::new();
    let value = match execute_anthropic_custom_base_fallback("glm-5", |transport| {
        seen.push(transport);
        async move {
            if matches!(transport, AnthropicCustomBaseTransport::OpenAi) {
                Ok::<_, &'static str>("ok")
            } else {
                Err("failed")
            }
        }
    })
    .await
    {
        Ok(value) => value,
        Err(error) => panic!("fallback should succeed on second transport: {error:?}"),
    };

    assert_eq!(value, "ok");
    assert_eq!(
        seen,
        vec![
            AnthropicCustomBaseTransport::Minimax,
            AnthropicCustomBaseTransport::OpenAi
        ]
    );
    Ok(())
}

#[tokio::test]
async fn execute_custom_base_fallback_returns_attempt_trace_when_exhausted() -> TestResult {
    let Err(failure) = execute_anthropic_custom_base_fallback(
        "claude-3-5-sonnet-20241022",
        |transport| async move {
            Err::<(), _>(anthropic_custom_base_transport_label(transport).to_string())
        },
    )
    .await
    else {
        panic!("fallback should fail when all transports fail");
    };

    assert_eq!(failure.attempts().len(), 3);
    assert_eq!(
        failure.last_error().map(String::as_str),
        Some("anthropic_messages_bypass")
    );
    Ok(())
}

#[test]
fn resolve_api_key_with_env_prefers_explicit_non_empty_value() {
    let resolved = resolve_api_key_with_env(
        Some("sk-explicit"),
        "XIUXIAN_TEST_PRIMARY_MISSING",
        "XIUXIAN_TEST_FALLBACK_MISSING",
    );
    assert_eq!(resolved.as_deref(), Some("sk-explicit"));
}

#[test]
fn resolve_api_key_with_env_returns_none_when_all_sources_absent_or_empty() {
    let resolved = resolve_api_key_with_env(
        Some("   "),
        "XIUXIAN_TEST_PRIMARY_MISSING",
        "XIUXIAN_TEST_FALLBACK_MISSING",
    );
    assert!(resolved.is_none());
}

#[test]
fn parse_positive_usize_accepts_positive_values() {
    assert_eq!(parse_positive_usize(Some("7"), 3), 7);
    assert_eq!(parse_positive_usize(Some(" 12 "), 3), 12);
}

#[test]
fn parse_positive_usize_falls_back_for_invalid_or_non_positive_values() {
    assert_eq!(parse_positive_usize(None, 3), 3);
    assert_eq!(parse_positive_usize(Some(""), 3), 3);
    assert_eq!(parse_positive_usize(Some("abc"), 3), 3);
    assert_eq!(parse_positive_usize(Some("0"), 3), 3);
    assert_eq!(parse_positive_usize(Some("-1"), 3), 3);
}

#[test]
fn resolve_positive_usize_env_uses_default_for_missing_env() {
    assert_eq!(
        resolve_positive_usize_env("XIUXIAN_TEST_POSITIVE_USIZE_ENV_MISSING", 5),
        5
    );
}

#[test]
fn resolve_required_api_key_with_env_returns_error_when_absent() {
    let Err(error) = resolve_required_api_key_with_env(
        None,
        "XIUXIAN_TEST_REQUIRED_PRIMARY_MISSING",
        "XIUXIAN_TEST_REQUIRED_FALLBACK_MISSING",
        "minimax",
    ) else {
        panic!("missing key should return error");
    };
    let rendered = error.to_string();
    assert!(rendered.contains("missing minimax api key"));
}

#[test]
fn resolve_required_api_key_with_env_prefers_explicit_value() {
    let result = resolve_required_api_key_with_env(
        Some("sk-required"),
        "XIUXIAN_TEST_REQUIRED_PRIMARY_MISSING",
        "XIUXIAN_TEST_REQUIRED_FALLBACK_MISSING",
        "anthropic",
    );
    assert_eq!(result.ok().as_deref(), Some("sk-required"));
}

#[cfg(feature = "provider-litellm")]
#[tokio::test]
async fn openai_provider_rejects_non_openai_key_prefix() -> TestResult {
    let result = build_openai_provider(
        "https://api.openai.com/v1".to_string(),
        Some("not-openai-prefixed-key".to_string()),
        30,
    )
    .await;
    assert!(
        result.is_err(),
        "strict OpenAI provider should reject non-sk key"
    );
    let rendered = match result {
        Ok(_) => String::new(),
        Err(error) => error.to_string(),
    };
    assert!(
        rendered.contains("OpenAI API key should start"),
        "error should expose key prefix validation, got: {rendered}"
    );
    Ok(())
}

#[cfg(feature = "provider-litellm")]
#[tokio::test]
async fn openai_like_provider_accepts_non_openai_key_prefix_for_custom_base() -> TestResult {
    let result = build_openai_like_provider(
        "https://proxy.example.com/api".to_string(),
        Some("not-openai-prefixed-key".to_string()),
        30,
    )
    .await;
    assert!(
        result.is_ok(),
        "openai-like provider should allow non-sk key for custom base: {result:?}"
    );
    Ok(())
}

#[cfg(feature = "provider-litellm")]
#[tokio::test]
async fn inline_openai_compatible_image_urls_converts_and_caches_image_urls() -> TestResult {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use axum::Router;
    use axum::body::Bytes;
    use axum::http::StatusCode;
    use axum::routing::get;
    use litellm_rs::core::types::chat::{ChatMessage, ChatRequest as LiteChatRequest};
    use litellm_rs::core::types::content::{ContentPart, ImageUrl};
    use litellm_rs::core::types::message::{MessageContent, MessageRole};
    use tokio::net::TcpListener;

    let fetch_count = Arc::new(AtomicUsize::new(0));
    let fetch_count_for_handler = Arc::clone(&fetch_count);
    let app = Router::new().route(
        "/img.png",
        get(move || {
            let fetch_count = Arc::clone(&fetch_count_for_handler);
            async move {
                fetch_count.fetch_add(1, Ordering::SeqCst);
                (
                    StatusCode::OK,
                    [("content-type", "image/png")],
                    Bytes::from_static(b"\x89PNG\r\n\x1a\nmock"),
                )
            }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let image_url = format!("http://{addr}/img.png");
    let request = LiteChatRequest {
        model: "glm-5".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: Some(MessageContent::Parts(vec![
                ContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: image_url.clone(),
                        detail: Some("high".to_string()),
                    },
                },
                ContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: image_url.clone(),
                        detail: Some("high".to_string()),
                    },
                },
            ])),
            ..Default::default()
        }],
        ..Default::default()
    };
    let client = reqwest::Client::new();
    let normalized = inline_openai_compatible_image_urls(&client, &request).await?;

    server.abort();

    let Some(MessageContent::Parts(parts)) = normalized
        .messages
        .first()
        .and_then(|message| message.content.clone())
    else {
        panic!("normalized request should keep multimodal parts");
    };
    assert_eq!(parts.len(), 2);
    for part in parts {
        match part {
            ContentPart::Image {
                source,
                detail,
                image_url,
            } => {
                assert_eq!(source.media_type, "image/png");
                assert!(!source.data.trim().is_empty());
                assert_eq!(detail.as_deref(), Some("high"));
                assert!(image_url.is_none());
            }
            other => panic!("expected inline image part, got {other:?}"),
        }
    }
    assert_eq!(
        fetch_count.load(Ordering::SeqCst),
        1,
        "same image URL should be fetched once then served from cache"
    );
    Ok(())
}

#[cfg(feature = "provider-litellm")]
#[tokio::test]
async fn inline_openai_compatible_image_urls_normalizes_octet_stream_data_uri() -> TestResult {
    use litellm_rs::core::types::chat::{ChatMessage, ChatRequest as LiteChatRequest};
    use litellm_rs::core::types::content::{ContentPart, ImageUrl};
    use litellm_rs::core::types::message::{MessageContent, MessageRole};

    let request = LiteChatRequest {
        model: "gpt-5-codex".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: Some(MessageContent::Parts(vec![ContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "data:application/octet-stream;base64,iVBORw0KGgoBAgM=".to_string(),
                    detail: Some("auto".to_string()),
                },
            }])),
            ..Default::default()
        }],
        ..Default::default()
    };

    let normalized = inline_openai_compatible_image_urls(&reqwest::Client::new(), &request).await?;
    let Some(MessageContent::Parts(parts)) = normalized
        .messages
        .first()
        .and_then(|message| message.content.clone())
    else {
        panic!("normalized request should keep multimodal parts");
    };

    assert_eq!(parts.len(), 1);
    match &parts[0] {
        ContentPart::Image {
            source,
            detail,
            image_url,
        } => {
            assert_eq!(source.media_type, "image/png");
            assert_eq!(source.data, "iVBORw0KGgoBAgM=");
            assert_eq!(detail.as_deref(), Some("auto"));
            assert!(image_url.is_none());
        }
        other => panic!("expected inline image part, got {other:?}"),
    }
    Ok(())
}

#[test]
fn normalize_optional_base_override_trims_and_drops_empty_values() {
    assert_eq!(
        normalize_optional_base_override(Some(" https://api.example.com/v1 ")).as_deref(),
        Some("https://api.example.com/v1")
    );
    assert_eq!(
        normalize_optional_base_override(Some(" https://api.example.com/api ")).as_deref(),
        Some("https://api.example.com/api/v1")
    );
    assert_eq!(
        normalize_optional_base_override(Some("https://api.example.com/v1/chat/completions"))
            .as_deref(),
        Some("https://api.example.com/v1")
    );
    assert!(normalize_optional_base_override(Some("   ")).is_none());
    assert!(normalize_optional_base_override(None).is_none());
}

#[cfg(feature = "provider-litellm")]
#[test]
fn build_anthropic_messages_body_from_request_preserves_core_fields() {
    use litellm_rs::core::types::chat::ChatRequest as LiteChatRequest;

    use xiuxian_llm::llm::providers::build_anthropic_messages_body_from_request;

    let request = LiteChatRequest {
        model: "claude-3-7-sonnet".to_string(),
        max_tokens: Some(1024),
        temperature: Some(0.2),
        top_p: Some(0.95),
        stop: Some(vec!["</done>".to_string()]),
        ..Default::default()
    };
    let messages = vec![json!({"role":"user","content":"hello"})];
    let body = build_anthropic_messages_body_from_request(
        &request,
        messages.as_slice(),
        Some("system prompt".to_string()),
    );

    assert_eq!(body["model"], json!("claude-3-7-sonnet"));
    assert_eq!(body["max_tokens"], json!(1024));
    assert_eq!(body["system"], json!("system prompt"));
    let Some(temperature) = body["temperature"].as_f64() else {
        panic!("temperature should serialize as number");
    };
    let Some(top_p) = body["top_p"].as_f64() else {
        panic!("top_p should serialize as number");
    };
    assert!((temperature - 0.2).abs() < 1e-5);
    assert!((top_p - 0.95).abs() < 1e-5);
    assert_eq!(body["stop_sequences"], json!(["</done>"]));
}

#[cfg(feature = "provider-litellm")]
#[test]
fn split_anthropic_system_messages_extracts_system_prompt() {
    use litellm_rs::core::types::chat::ChatMessage;
    use litellm_rs::core::types::message::{MessageContent, MessageRole};

    use xiuxian_llm::llm::providers::split_anthropic_system_messages;

    let messages = vec![
        ChatMessage {
            role: MessageRole::System,
            content: Some(MessageContent::Text("policy".to_string())),
            ..Default::default()
        },
        ChatMessage {
            role: MessageRole::User,
            content: Some(MessageContent::Text("hello".to_string())),
            ..Default::default()
        },
    ];

    let (system, others) = split_anthropic_system_messages(messages.as_slice());
    assert_eq!(system.as_deref(), Some("policy"));
    assert_eq!(others.len(), 1);
    assert_eq!(others[0].role, MessageRole::User);
}

#[cfg(feature = "provider-litellm")]
#[tokio::test]
async fn build_anthropic_messages_body_from_litellm_request_with_image_hook_injects_overlay()
-> TestResult {
    use litellm_rs::core::types::chat::{ChatMessage, ChatRequest as LiteChatRequest};
    use litellm_rs::core::types::content::{ContentPart, ImageSource};
    use litellm_rs::core::types::message::{MessageContent, MessageRole};

    use xiuxian_llm::llm::providers::build_anthropic_messages_body_from_litellm_request_with_image_hook;

    let request = LiteChatRequest {
        model: "claude-3-7-sonnet".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: Some(MessageContent::Parts(vec![ContentPart::Image {
                source: ImageSource {
                    media_type: "image/png".to_string(),
                    data: "iVBORw0KGgo=".to_string(),
                },
                detail: None,
                image_url: None,
            }])),
            ..Default::default()
        }],
        ..Default::default()
    };
    let client = reqwest::Client::new();
    let body = build_anthropic_messages_body_from_litellm_request_with_image_hook(
        &client,
        &request,
        |source| async move { Some(format!("ocr:{}", source.media_type)) },
    )
    .await?;

    assert_eq!(body["messages"][0]["content"][0]["type"], json!("text"));
    assert_eq!(
        body["messages"][0]["content"][0]["text"],
        json!("ocr:image/png")
    );
    assert_eq!(body["messages"][0]["content"][1]["type"], json!("image"));
    assert_eq!(
        body["messages"][0]["content"][1]["source"]["media_type"],
        json!("image/png")
    );
    Ok(())
}

#[cfg(feature = "provider-litellm")]
#[tokio::test]
async fn build_anthropic_messages_body_maps_tool_call_chain_to_tool_use_and_tool_result()
-> TestResult {
    use litellm_rs::core::types::chat::{ChatMessage, ChatRequest as LiteChatRequest};
    use litellm_rs::core::types::message::{MessageContent, MessageRole};
    use litellm_rs::core::types::tools::{FunctionCall, ToolCall};

    use xiuxian_llm::llm::providers::build_anthropic_messages_body_from_litellm_request_with_image_hook;

    let request = LiteChatRequest {
        model: "claude-3-7-sonnet".to_string(),
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: Some(MessageContent::Text("hello".to_string())),
                ..Default::default()
            },
            ChatMessage {
                role: MessageRole::Assistant,
                tool_calls: Some(vec![ToolCall {
                    id: "call_1".to_string(),
                    tool_type: "function".to_string(),
                    function: FunctionCall {
                        name: "search".to_string(),
                        arguments: r#"{"q":"rust"}"#.to_string(),
                    },
                }]),
                ..Default::default()
            },
            ChatMessage {
                role: MessageRole::Tool,
                tool_call_id: Some("call_1".to_string()),
                content: Some(MessageContent::Text("tool-result".to_string())),
                ..Default::default()
            },
        ],
        ..Default::default()
    };
    let client = reqwest::Client::new();
    let body = build_anthropic_messages_body_from_litellm_request_with_image_hook(
        &client,
        &request,
        |_source| async move { None::<String> },
    )
    .await?;

    assert_eq!(body["messages"][1]["role"], json!("assistant"));
    assert_eq!(body["messages"][1]["content"][0]["type"], json!("tool_use"));
    assert_eq!(body["messages"][1]["content"][0]["id"], json!("call_1"));
    assert_eq!(body["messages"][1]["content"][0]["name"], json!("search"));
    assert_eq!(body["messages"][2]["role"], json!("user"));
    assert_eq!(
        body["messages"][2]["content"][0]["type"],
        json!("tool_result")
    );
    assert_eq!(
        body["messages"][2]["content"][0]["tool_use_id"],
        json!("call_1")
    );
    assert_eq!(
        body["messages"][2]["content"][0]["content"],
        json!("tool-result")
    );
    Ok(())
}

#[cfg(feature = "provider-litellm")]
#[tokio::test]
async fn execute_anthropic_messages_with_image_hook_round_trips_response_and_request_shape()
-> TestResult {
    use std::sync::Arc;

    use axum::Router;
    use axum::extract::{Json, State};
    use axum::routing::post;
    use litellm_rs::core::types::chat::{ChatMessage, ChatRequest as LiteChatRequest};
    use litellm_rs::core::types::content::{ContentPart, ImageSource};
    use litellm_rs::core::types::message::{MessageContent, MessageRole};
    use tokio::net::TcpListener;
    use tokio::sync::Mutex;

    type CapturedRequest = Arc<Mutex<Option<serde_json::Value>>>;

    async fn handler(
        State(captured): State<CapturedRequest>,
        Json(payload): Json<serde_json::Value>,
    ) -> Json<serde_json::Value> {
        *captured.lock().await = Some(payload);
        Json(json!({
            "content": [
                {"type": "text", "text": "ack"},
                {"type": "tool_use", "id": "call_1", "name": "search", "input": {"q": "rust"}}
            ]
        }))
    }

    let captured: CapturedRequest = Arc::new(Mutex::new(None));
    let app = Router::new()
        .route("/v1/messages", post(handler))
        .with_state(Arc::clone(&captured));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    let request = LiteChatRequest {
        model: "claude-3-7-sonnet".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: Some(MessageContent::Parts(vec![
                ContentPart::Text {
                    text: "hello".to_string(),
                },
                ContentPart::Image {
                    source: ImageSource {
                        media_type: "image/png".to_string(),
                        data: "iVBORw0KGgo=".to_string(),
                    },
                    detail: None,
                    image_url: None,
                },
            ])),
            ..Default::default()
        }],
        ..Default::default()
    };
    let endpoint = format!("http://{addr}/v1/messages");
    let client = reqwest::Client::new();
    let parsed = execute_anthropic_messages_from_litellm_request_with_image_hook(
        &client,
        endpoint.as_str(),
        "sk-test",
        &request,
        1,
        |_source| async move { Some("ocr-overlay".to_string()) },
    )
    .await?;

    assert_eq!(parsed.text.as_deref(), Some("ack"));
    assert_eq!(parsed.tool_uses.len(), 1);
    assert_eq!(parsed.tool_uses[0].name, "search");
    let Some(captured_request) = captured.lock().await.clone() else {
        panic!("server should capture request");
    };
    assert_eq!(
        captured_request["messages"][0]["content"][0]["text"],
        json!("hello")
    );
    assert_eq!(
        captured_request["messages"][0]["content"][1]["text"],
        json!("ocr-overlay")
    );
    assert_eq!(
        captured_request["messages"][0]["content"][2]["type"],
        json!("image")
    );
    Ok(())
}

#[cfg(feature = "provider-litellm")]
#[test]
fn build_anthropic_messages_body_normalizes_image_media_type() {
    use litellm_rs::core::types::chat::ChatRequest as LiteChatRequest;

    use xiuxian_llm::llm::providers::build_anthropic_messages_body_from_request;

    let request = LiteChatRequest {
        model: "claude-3-7-sonnet".to_string(),
        max_tokens: Some(256),
        ..Default::default()
    };
    let messages = vec![json!({
        "role": "user",
        "content": [{
            "type": "image",
            "source": {
                "type": "base64",
                "media_type": "application/octet-stream",
                "data": "/9j/2w=="
            }
        }]
    })];
    let body = build_anthropic_messages_body_from_request(&request, messages.as_slice(), None);
    assert_eq!(
        body["messages"][0]["content"][0]["source"]["media_type"],
        json!("image/jpeg")
    );
}

#[cfg(feature = "provider-litellm")]
#[test]
fn build_anthropic_messages_body_maps_tool_choice_required_to_any() {
    use litellm_rs::core::types::chat::ChatRequest as LiteChatRequest;
    use litellm_rs::core::types::tools::{FunctionDefinition, Tool, ToolChoice, ToolType};

    use xiuxian_llm::llm::providers::build_anthropic_messages_body_from_request;

    let request = LiteChatRequest {
        model: "claude-3-7-sonnet".to_string(),
        tools: Some(vec![Tool {
            tool_type: ToolType::Function,
            function: FunctionDefinition {
                name: "search".to_string(),
                description: Some("search docs".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": { "q": { "type": "string" } },
                    "required": ["q"]
                })),
            },
        }]),
        tool_choice: Some(ToolChoice::String("required".to_string())),
        ..Default::default()
    };
    let messages = vec![json!({"role":"user","content":"hello"})];
    let body = build_anthropic_messages_body_from_request(&request, messages.as_slice(), None);
    assert_eq!(body["tool_choice"], json!({"type":"any"}));
    assert_eq!(body["tools"][0]["name"], json!("search"));
}

#[cfg(feature = "provider-litellm")]
#[tokio::test]
async fn openai_transform_request_preserves_multimodal_tools_and_response_format() -> TestResult {
    use litellm_rs::core::traits::provider::llm_provider::trait_definition::LLMProvider;
    use litellm_rs::core::types::chat::{ChatMessage, ChatRequest as LiteChatRequest};
    use litellm_rs::core::types::content::{ContentPart, ImageSource, ImageUrl};
    use litellm_rs::core::types::context::RequestContext as LiteRequestContext;
    use litellm_rs::core::types::message::{MessageContent, MessageRole};
    use litellm_rs::core::types::tools::{
        FunctionChoice, FunctionDefinition, ResponseFormat, Tool, ToolChoice, ToolType,
    };

    use xiuxian_llm::llm::providers::build_openai_provider;

    let provider = build_openai_provider(
        "https://api.openai.com/v1".to_string(),
        Some("sk-test".to_string()),
        30,
    )
    .await?;

    let request = LiteChatRequest {
        model: "gpt-4o".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: Some(MessageContent::Parts(vec![
                ContentPart::Text {
                    text: "read this image".to_string(),
                },
                ContentPart::Image {
                    source: ImageSource {
                        media_type: "image/png".to_string(),
                        data: "iVBORw0KGgo=".to_string(),
                    },
                    detail: Some("high".to_string()),
                    image_url: Some(ImageUrl {
                        url: "data:image/png;base64,iVBORw0KGgo=".to_string(),
                        detail: Some("high".to_string()),
                    }),
                },
            ])),
            ..Default::default()
        }],
        tools: Some(vec![Tool {
            tool_type: ToolType::Function,
            function: FunctionDefinition {
                name: "search".to_string(),
                description: Some("search docs".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": { "q": { "type": "string" } }
                })),
            },
        }]),
        tool_choice: Some(ToolChoice::Specific {
            choice_type: "function".to_string(),
            function: Some(FunctionChoice {
                name: "search".to_string(),
            }),
        }),
        response_format: Some(ResponseFormat {
            format_type: "json_object".to_string(),
            json_schema: None,
            response_type: None,
        }),
        max_tokens: Some(256),
        temperature: Some(0.2),
        user: Some("user-1".to_string()),
        ..Default::default()
    };

    let payload =
        LLMProvider::transform_request(&provider, request, LiteRequestContext::new()).await?;

    assert_eq!(payload["model"], json!("gpt-4o"));
    assert_eq!(payload["messages"][0]["role"], json!("user"));
    assert_eq!(payload["messages"][0]["content"][1]["type"], json!("image"));
    assert_eq!(
        payload["messages"][0]["content"][1]["source"]["media_type"],
        json!("image/png")
    );
    assert_eq!(payload["tools"][0]["type"], json!("function"));
    assert_eq!(payload["tool_choice"]["type"], json!("function"));
    assert_eq!(payload["tool_choice"]["function"]["name"], json!("search"));
    assert_eq!(payload["response_format"]["type"], json!("json_object"));
    Ok(())
}

#[cfg(feature = "provider-litellm")]
#[tokio::test]
async fn openai_transform_request_keeps_proxy_model_id_without_registry_validation() -> TestResult {
    use litellm_rs::core::traits::provider::llm_provider::trait_definition::LLMProvider;
    use litellm_rs::core::types::chat::{ChatMessage, ChatRequest as LiteChatRequest};
    use litellm_rs::core::types::context::RequestContext as LiteRequestContext;
    use litellm_rs::core::types::message::{MessageContent, MessageRole};

    use xiuxian_llm::llm::providers::build_openai_provider;

    let provider = build_openai_provider(
        "https://api.openai.com/v1".to_string(),
        Some("sk-test".to_string()),
        30,
    )
    .await?;
    let request = LiteChatRequest {
        model: "glm-5".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: Some(MessageContent::Text("hello".to_string())),
            ..Default::default()
        }],
        ..Default::default()
    };

    let payload =
        LLMProvider::transform_request(&provider, request, LiteRequestContext::new()).await?;
    assert_eq!(payload["model"], json!("glm-5"));
    Ok(())
}

#[cfg(feature = "provider-litellm")]
#[tokio::test]
async fn anthropic_transform_request_rejects_unknown_model_before_transport() -> TestResult {
    use litellm_rs::core::traits::provider::llm_provider::trait_definition::LLMProvider;
    use litellm_rs::core::types::chat::{ChatMessage, ChatRequest as LiteChatRequest};
    use litellm_rs::core::types::context::RequestContext as LiteRequestContext;
    use litellm_rs::core::types::message::{MessageContent, MessageRole};

    use xiuxian_llm::llm::providers::build_anthropic_provider;

    let provider = build_anthropic_provider(
        "https://proxy.example.com/api".to_string(),
        "sk-test".to_string(),
        30,
    )
    .await?;

    let request = LiteChatRequest {
        model: "glm-5".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: Some(MessageContent::Text("hello".to_string())),
            ..Default::default()
        }],
        ..Default::default()
    };

    let Err(error) =
        LLMProvider::transform_request(&provider, request, LiteRequestContext::new()).await
    else {
        panic!("unknown anthropic model should fail fast");
    };
    let error_text = error.to_string();
    assert!(
        error_text.contains("Unsupported model: glm-5"),
        "unexpected error: {error_text}"
    );
    Ok(())
}
