//! Integration tests for OpenAI-compatible chat client behavior.

use anyhow::{Result, anyhow};
use axum::extract::State;
use axum::http::{StatusCode, header::CONTENT_TYPE};
use axum::routing::post;
use axum::{Router, response::IntoResponse};
use tokio::net::TcpListener;
use xiuxian_llm::llm::{
    ChatMessage, ChatRequest, ContentPart, ImageUrlContent, LlmClient, MessageContent, OpenAIClient,
};

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

async fn spawn_mock_openai_server(state: MockResponse) -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let app = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok(format!("http://{addr}/v1"))
}

async fn spawn_mock_openai_server_without_v1_base(state: MockResponse) -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let app = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state);
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok(format!("http://{addr}"))
}

fn request() -> ChatRequest {
    ChatRequest {
        model: "test-model".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "hello".into(),
        }],
        temperature: 0.1,
    }
}

#[test]
fn chat_request_serializes_multimodal_content_parts() -> Result<()> {
    let request = ChatRequest {
        model: "test-model".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: MessageContent::Parts(vec![
                ContentPart::Text {
                    text: "What is in this image?".to_string(),
                },
                ContentPart::ImageUrl {
                    image_url: ImageUrlContent {
                        url: "data:image/jpeg;base64,AAEC".to_string(),
                        detail: "high".to_string(),
                    },
                },
            ]),
        }],
        temperature: 0.1,
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
    let base_url = spawn_mock_openai_server(MockResponse {
        status: StatusCode::OK,
        content_type: "application/json",
        body: r#"{
          "choices": [
            {
              "message": {
                "role": "assistant",
                "content": "ok"
              }
            }
          ]
        }"#,
    })
    .await?;

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
    let base_url = spawn_mock_openai_server(MockResponse {
        status: StatusCode::BAD_REQUEST,
        content_type: "application/json",
        body: r#"{"error":{"message":"model not found"}}"#,
    })
    .await?;

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
    let base_url = spawn_mock_openai_server(MockResponse {
        status: StatusCode::OK,
        content_type: "text/plain",
        body: "upstream unavailable",
    })
    .await?;

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
        text.contains("LLM response decoding failed"),
        "unexpected error: {text}"
    );
    assert!(
        text.contains("upstream unavailable"),
        "unexpected error: {text}"
    );
    Ok(())
}

#[tokio::test]
async fn openai_client_chat_retries_with_v1_fallback_after_404() -> Result<()> {
    let base_url = spawn_mock_openai_server_without_v1_base(MockResponse {
        status: StatusCode::OK,
        content_type: "application/json",
        body: r#"{
          "choices": [
            {
              "message": {
                "role": "assistant",
                "content": "fallback-ok"
              }
            }
          ]
        }"#,
    })
    .await?;

    let client = OpenAIClient {
        api_key: "test".to_string(),
        base_url,
        http: reqwest::Client::new(),
    };

    let result = client.chat(request()).await?;
    assert_eq!(result, "fallback-ok");
    Ok(())
}
