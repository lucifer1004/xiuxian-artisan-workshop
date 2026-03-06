//! Integration tests for OpenAI-compatible chat client behavior.
#![cfg(feature = "provider-litellm")]

use anyhow::{Result, anyhow};
use axum::extract::Json;
use axum::extract::State;
use axum::http::{StatusCode, header::CONTENT_TYPE};
use axum::routing::post;
use axum::{Router, response::IntoResponse};
use tokio::net::TcpListener;
use xiuxian_llm::llm::{
    ChatMessage, ChatRequest, ContentPart, ImageUrlContent, LlmClient, MessageContent, MessageRole,
    OpenAIClient, OpenAICompatibleClient, OpenAIWireApi,
};

struct MockServer {
    base_url: String,
    task: tokio::task::JoinHandle<()>,
}

impl MockServer {
    fn base_url(&self) -> String {
        self.base_url.clone()
    }
}

impl Drop for MockServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[derive(Clone)]
struct MockResponse {
    status: StatusCode,
    content_type: &'static str,
    body: &'static str,
}

async fn chat_completions(State(state): State<MockResponse>) -> impl IntoResponse {
    (
        state.status,
        [(CONTENT_TYPE, state.content_type)],
        state.body.to_string(),
    )
}

async fn responses_requires_stream(Json(payload): Json<serde_json::Value>) -> impl IntoResponse {
    let requires_stream = payload
        .get("stream")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if requires_stream {
        return (
            StatusCode::OK,
            [(CONTENT_TYPE, "text/event-stream")],
            r#"data: {"type":"response.output_item.done","item":{"type":"message","content":[{"type":"output_text","text":"stream-retry-ok"}]}}
data: [DONE]"#
                .to_string(),
        );
    }

    (
        StatusCode::BAD_REQUEST,
        [(CONTENT_TYPE, "application/json")],
        r#"{"error":{"message":"Stream must be set to true"}}"#.to_string(),
    )
}

async fn chat_completions_requires_stream(
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    let requires_stream = payload
        .get("stream")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    if requires_stream {
        return (
            StatusCode::OK,
            [(CONTENT_TYPE, "text/event-stream")],
            r#"data: {"id":"chatcmpl-1","object":"chat.completion.chunk","created":0,"model":"test-model","choices":[{"index":0,"delta":{"content":"stream-chat-ok"},"finish_reason":null}]}

data: [DONE]
"#
                .to_string(),
        );
    }

    (
        StatusCode::BAD_REQUEST,
        [(CONTENT_TYPE, "application/json")],
        r#"{"error":{"message":"Stream must be set to true"}}"#.to_string(),
    )
}

async fn spawn_mock_openai_server(state: MockResponse) -> Result<MockServer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let app = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/responses", post(chat_completions))
        .with_state(state);
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok(MockServer {
        base_url: format!("http://{addr}/v1"),
        task,
    })
}

async fn spawn_mock_openai_server_without_v1_base(state: MockResponse) -> Result<MockServer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let app = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/responses", post(chat_completions))
        .with_state(state);
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok(MockServer {
        base_url: format!("http://{addr}"),
        task,
    })
}

async fn spawn_mock_openai_server_responses_requires_stream() -> Result<MockServer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let app = Router::new().route("/v1/responses", post(responses_requires_stream));
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok(MockServer {
        base_url: format!("http://{addr}/v1"),
        task,
    })
}

async fn spawn_mock_openai_server_chat_requires_stream() -> Result<MockServer> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let app = Router::new().route(
        "/v1/chat/completions",
        post(chat_completions_requires_stream),
    );
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok(MockServer {
        base_url: format!("http://{addr}/v1"),
        task,
    })
}

fn request() -> ChatRequest {
    ChatRequest {
        model: "test-model".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: Some("hello".into()),
            ..ChatMessage::default()
        }],
        temperature: Some(0.1),
        ..ChatRequest::default()
    }
}

#[test]
fn chat_request_serializes_multimodal_content_parts() -> Result<()> {
    let request = ChatRequest {
        model: "test-model".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: Some(MessageContent::Parts(vec![
                ContentPart::Text {
                    text: "What is in this image?".to_string(),
                },
                ContentPart::ImageUrl {
                    image_url: ImageUrlContent {
                        url: "data:image/jpeg;base64,AAEC".to_string(),
                        detail: Some("high".to_string()),
                    },
                },
            ])),
            ..ChatMessage::default()
        }],
        temperature: Some(0.1),
        ..ChatRequest::default()
    };

    let payload = serde_json::to_value(&request)?;
    assert_eq!(payload["messages"][0]["content"][0]["type"], "text");
    assert_eq!(
        payload["messages"][0]["content"][0]["text"],
        "What is in this image?"
    );
    assert_eq!(payload["messages"][0]["content"][1]["type"], "image_url");
    assert_eq!(
        payload["messages"][0]["content"][1]["image_url"]["url"],
        "data:image/jpeg;base64,AAEC"
    );
    assert_eq!(
        payload["messages"][0]["content"][1]["image_url"]["detail"],
        "high"
    );
    Ok(())
}

#[tokio::test]
async fn openai_client_chat_success_returns_first_choice_content() -> Result<()> {
    let server = spawn_mock_openai_server(MockResponse {
        status: StatusCode::OK,
        content_type: "application/json",
        body: r#"{
          "id": "chatcmpl-success",
          "object": "chat.completion",
          "created": 0,
          "model": "test-model",
          "choices": [
            {
              "index": 0,
              "message": {
                "role": "assistant",
                "content": "ok"
              },
              "finish_reason": "stop"
            }
          ]
        }"#,
    })
    .await?;
    let base_url = server.base_url();

    let client = OpenAIClient {
        api_key: "test".to_string(),
        base_url,
        http: reqwest::Client::new(),
    };

    let result = client.chat(request()).await?;
    assert_eq!(result, "ok");
    Ok(())
}

#[tokio::test]
async fn openai_client_chat_non_success_status_surfaces_provider_message() -> Result<()> {
    let server = spawn_mock_openai_server(MockResponse {
        status: StatusCode::BAD_REQUEST,
        content_type: "application/json",
        body: r#"{"error":{"message":"model not found"}}"#,
    })
    .await?;
    let base_url = server.base_url();

    let client = OpenAIClient {
        api_key: "test".to_string(),
        base_url,
        http: reqwest::Client::new(),
    };

    let Err(err) = client.chat(request()).await else {
        return Err(anyhow!("chat should fail"));
    };
    let text = err.to_string();
    assert!(text.contains("status 400"), "unexpected error: {text}");
    assert!(text.contains("model not found"), "unexpected error: {text}");
    Ok(())
}

#[tokio::test]
async fn openai_client_chat_decode_error_includes_body_preview() -> Result<()> {
    let server = spawn_mock_openai_server(MockResponse {
        status: StatusCode::OK,
        content_type: "text/plain",
        body: "upstream unavailable",
    })
    .await?;
    let base_url = server.base_url();

    let client = OpenAIClient {
        api_key: "test".to_string(),
        base_url,
        http: reqwest::Client::new(),
    };

    let Err(err) = client.chat(request()).await else {
        return Err(anyhow!("chat should fail"));
    };
    let text = err.to_string();
    assert!(
        text.contains("LLM response decoding failed")
            || text.contains("litellm-rs openai_like chat completion failed"),
        "unexpected error: {text}"
    );
    assert!(
        text.contains("upstream unavailable") || text.contains("expected value at line 1 column 1"),
        "unexpected error: {text}"
    );
    Ok(())
}

#[tokio::test]
async fn openai_client_chat_retries_with_v1_fallback_after_404() -> Result<()> {
    let server = spawn_mock_openai_server_without_v1_base(MockResponse {
        status: StatusCode::OK,
        content_type: "application/json",
        body: r#"{
          "id": "chatcmpl-fallback-openai-client",
          "object": "chat.completion",
          "created": 0,
          "model": "test-model",
          "choices": [
            {
              "index": 0,
              "message": {
                "role": "assistant",
                "content": "fallback-ok"
              },
              "finish_reason": "stop"
            }
          ]
        }"#,
    })
    .await?;
    let base_url = server.base_url();

    let client = OpenAIClient {
        api_key: "test".to_string(),
        base_url,
        http: reqwest::Client::new(),
    };

    let result = client.chat(request()).await?;
    assert_eq!(result, "fallback-ok");
    Ok(())
}

#[tokio::test]
async fn openai_compatible_client_responses_success_returns_output_text() -> Result<()> {
    let body = r#"data: {"type":"response.output_item.done","item":{"type":"message","content":[{"type":"output_text","text":"responses-ok"}]}}
data: [DONE]"#;

    let server = spawn_mock_openai_server(MockResponse {
        status: StatusCode::OK,
        content_type: "text/event-stream",
        body,
    })
    .await?;
    let base_url = server.base_url();

    let client = OpenAICompatibleClient {
        api_key: "test".to_string(),
        base_url,
        wire_api: OpenAIWireApi::Responses,
        http: reqwest::Client::new(),
    };

    let result = client.chat(request()).await?;
    assert_eq!(result, "responses-ok");
    Ok(())
}

#[tokio::test]
async fn openai_compatible_client_responses_retries_with_v1_fallback_after_404() -> Result<()> {
    let body = r#"data: {"type":"response.output_item.done","item":{"type":"message","content":[{"type":"output_text","text":"responses-fallback-ok"}]}}
data: [DONE]"#;

    let server = spawn_mock_openai_server_without_v1_base(MockResponse {
        status: StatusCode::OK,
        content_type: "text/event-stream",
        body,
    })
    .await?;
    let base_url = server.base_url();

    let client = OpenAICompatibleClient {
        api_key: "test".to_string(),
        base_url,
        wire_api: OpenAIWireApi::Responses,
        http: reqwest::Client::new(),
    };

    let result = client.chat(request()).await?;
    assert_eq!(result, "responses-fallback-ok");
    Ok(())
}

#[tokio::test]
async fn openai_compatible_client_responses_retries_with_stream_transport_when_required()
-> Result<()> {
    let server = spawn_mock_openai_server_responses_requires_stream().await?;
    let base_url = server.base_url();

    let client = OpenAICompatibleClient {
        api_key: "test".to_string(),
        base_url,
        wire_api: OpenAIWireApi::Responses,
        http: reqwest::Client::new(),
    };

    let result = client.chat(request()).await?;
    assert_eq!(result, "stream-retry-ok");
    Ok(())
}

#[tokio::test]
async fn openai_compatible_client_chat_retries_with_stream_transport_when_required() -> Result<()> {
    let server = spawn_mock_openai_server_chat_requires_stream().await?;
    let base_url = server.base_url();

    let client = OpenAICompatibleClient {
        api_key: "test".to_string(),
        base_url,
        wire_api: OpenAIWireApi::ChatCompletions,
        http: reqwest::Client::new(),
    };

    let result = client.chat(request()).await?;
    assert_eq!(result, "stream-chat-ok");
    Ok(())
}

#[tokio::test]
async fn openai_compatible_client_chat_retries_with_v1_fallback_after_404() -> Result<()> {
    let server = spawn_mock_openai_server_without_v1_base(MockResponse {
        status: StatusCode::OK,
        content_type: "application/json",
        body: r#"{
          "id": "chatcmpl-fallback",
          "object": "chat.completion",
          "created": 0,
          "model": "test-model",
          "choices": [
            {
              "index": 0,
              "message": {
                "role": "assistant",
                "content": "chat-fallback-ok"
              },
              "finish_reason": "stop"
            }
          ]
        }"#,
    })
    .await?;
    let base_url = server.base_url();

    let client = OpenAICompatibleClient {
        api_key: "test".to_string(),
        base_url,
        wire_api: OpenAIWireApi::ChatCompletions,
        http: reqwest::Client::new(),
    };

    let result = client.chat(request()).await?;
    assert_eq!(result, "chat-fallback-ok");
    Ok(())
}
