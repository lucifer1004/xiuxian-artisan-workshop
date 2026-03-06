#![cfg(feature = "provider-litellm")]

//! Regression tests for OpenAI `/responses` payload normalization.

use litellm_rs::core::types::chat::{ChatMessage, ChatRequest as LiteChatRequest};
use litellm_rs::core::types::content::ContentPart;
use litellm_rs::core::types::message::{MessageContent, MessageRole};
use litellm_rs::core::types::tools::{FunctionCall, FunctionDefinition, Tool, ToolCall, ToolType};
use serde_json::json;
use xiuxian_llm::llm::providers::build_openai_responses_payload;

#[test]
fn responses_payload_injects_empty_properties_for_object_schema_without_properties() {
    let request = LiteChatRequest {
        model: "gpt-5-codex".to_string(),
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: Some(MessageContent::Text("hello".to_string())),
            ..Default::default()
        }],
        tools: Some(vec![Tool {
            tool_type: ToolType::Function,
            function: FunctionDefinition {
                name: "qianhuan.reload".to_string(),
                description: Some("Reload qianhuan runtime".to_string()),
                parameters: Some(json!({
                    "type": "object"
                })),
            },
        }]),
        ..Default::default()
    };

    let payload = build_openai_responses_payload(&request).payload;
    assert_eq!(payload["tools"][0]["name"], json!("qianhuan_reload"));
    assert_eq!(payload["tools"][0]["parameters"]["type"], json!("object"));
    assert_eq!(payload["tools"][0]["parameters"]["properties"], json!({}));
}

#[test]
fn responses_payload_serializes_assistant_tool_calls_before_tool_outputs() {
    let request = LiteChatRequest {
        model: "gpt-5-codex".to_string(),
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
                        name: "qianhuan.reload".to_string(),
                        arguments: r#"{"scope":"all"}"#.to_string(),
                    },
                }]),
                ..Default::default()
            },
            ChatMessage {
                role: MessageRole::Tool,
                content: Some(MessageContent::Text("reloaded".to_string())),
                tool_call_id: Some("call_1".to_string()),
                ..Default::default()
            },
        ],
        tools: Some(vec![Tool {
            tool_type: ToolType::Function,
            function: FunctionDefinition {
                name: "qianhuan.reload".to_string(),
                description: Some("Reload qianhuan runtime".to_string()),
                parameters: Some(json!({
                    "type": "object",
                    "properties": {
                        "scope": { "type": "string" }
                    }
                })),
            },
        }]),
        ..Default::default()
    };

    let payload = build_openai_responses_payload(&request).payload;
    let input = payload["input"]
        .as_array()
        .expect("responses payload should include input array");
    let function_call_index = input
        .iter()
        .position(|item| {
            item.get("type").and_then(serde_json::Value::as_str) == Some("function_call")
        })
        .expect("assistant function_call should be emitted");
    let function_output_index = input
        .iter()
        .position(|item| {
            item.get("type").and_then(serde_json::Value::as_str) == Some("function_call_output")
        })
        .expect("tool output should be emitted");

    assert!(function_call_index < function_output_index);
    assert_eq!(input[function_call_index]["call_id"], json!("call_1"));
    assert_eq!(input[function_call_index]["name"], json!("qianhuan_reload"));
    assert_eq!(
        input[function_call_index]["arguments"],
        json!(r#"{"scope":"all"}"#)
    );
    assert_eq!(input[function_output_index]["call_id"], json!("call_1"));
    assert_eq!(input[function_output_index]["output"], json!("reloaded"));
}

#[test]
fn responses_payload_normalizes_pipe_delimited_tool_call_ids() {
    let request = LiteChatRequest {
        model: "gpt-5-codex".to_string(),
        messages: vec![
            ChatMessage {
                role: MessageRole::Assistant,
                tool_calls: Some(vec![ToolCall {
                    id: "call_1|fc_1".to_string(),
                    tool_type: "function".to_string(),
                    function: FunctionCall {
                        name: "qianhuan.reload".to_string(),
                        arguments: r#"{"scope":"all"}"#.to_string(),
                    },
                }]),
                ..Default::default()
            },
            ChatMessage {
                role: MessageRole::Tool,
                content: Some(MessageContent::Text("reloaded".to_string())),
                tool_call_id: Some("call_1|tool_output_1".to_string()),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let payload = build_openai_responses_payload(&request).payload;
    let input = payload["input"]
        .as_array()
        .expect("responses payload should include input array");
    let function_call = input
        .iter()
        .find(|item| item.get("type").and_then(serde_json::Value::as_str) == Some("function_call"))
        .expect("assistant function_call should be emitted");
    let function_output = input
        .iter()
        .find(|item| {
            item.get("type").and_then(serde_json::Value::as_str) == Some("function_call_output")
        })
        .expect("tool output should be emitted");

    assert_eq!(function_call["call_id"], json!("call_1"));
    assert_eq!(function_output["call_id"], json!("call_1"));
}

#[test]
fn responses_payload_skips_tool_output_without_call_id() {
    let request = LiteChatRequest {
        model: "gpt-5-codex".to_string(),
        messages: vec![
            ChatMessage {
                role: MessageRole::User,
                content: Some(MessageContent::Text("hello".to_string())),
                ..Default::default()
            },
            ChatMessage {
                role: MessageRole::Tool,
                content: Some(MessageContent::Text("orphan tool output".to_string())),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let payload = build_openai_responses_payload(&request).payload;
    let input = payload["input"]
        .as_array()
        .expect("responses payload should include input array");
    assert!(
        !input.iter().any(|item| {
            item.get("type").and_then(serde_json::Value::as_str) == Some("function_call_output")
        }),
        "tool output without call_id must be skipped to avoid invalid responses payload",
    );
}

#[test]
fn responses_payload_serializes_tool_result_parts_into_function_call_output() {
    let request = LiteChatRequest {
        model: "gpt-5-codex".to_string(),
        messages: vec![
            ChatMessage {
                role: MessageRole::Assistant,
                tool_calls: Some(vec![ToolCall {
                    id: "call_1".to_string(),
                    tool_type: "function".to_string(),
                    function: FunctionCall {
                        name: "qianhuan.reload".to_string(),
                        arguments: r#"{"scope":"all"}"#.to_string(),
                    },
                }]),
                ..Default::default()
            },
            ChatMessage {
                role: MessageRole::Tool,
                content: Some(MessageContent::Parts(vec![ContentPart::ToolResult {
                    tool_use_id: "call_1".to_string(),
                    content: json!({"ok": true}),
                    is_error: None,
                }])),
                tool_call_id: Some("call_1".to_string()),
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let payload = build_openai_responses_payload(&request).payload;
    let input = payload["input"]
        .as_array()
        .expect("responses payload should include input array");
    let function_output = input
        .iter()
        .find(|item| {
            item.get("type").and_then(serde_json::Value::as_str) == Some("function_call_output")
        })
        .expect("tool output should be emitted");

    assert_eq!(function_output["call_id"], json!("call_1"));
    assert_eq!(function_output["output"], json!(r#"{"ok":true}"#));
}
