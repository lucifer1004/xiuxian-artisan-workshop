use anyhow::Result;
use litellm_rs::core::types::chat::ChatMessage as LiteChatMessage;
use litellm_rs::core::types::content::{ContentPart as LiteContentPart, ImageUrl as LiteImageUrl};
use litellm_rs::core::types::message::{
    MessageContent as LiteMessageContent, MessageRole as LiteMessageRole,
};
use litellm_rs::core::types::tools::{FunctionCall as LiteFunctionCall, ToolCall as LiteToolCall};
use xiuxian_llm::llm::multimodal::{MultimodalContentPart, parse_multimodal_text_content};

use crate::session::{ChatMessage, FunctionCall, ToolCallOut};

fn normalize_tool_call_id(raw_id: &str) -> Option<&str> {
    raw_id
        .split('|')
        .next()
        .map(str::trim)
        .filter(|id| !id.is_empty())
}

fn chat_message_content_to_litellm(content: Option<String>) -> Option<LiteMessageContent> {
    let content = content?;
    if let Some(parts) = parse_multimodal_text_content(content.as_str()) {
        let litellm_parts = parts
            .into_iter()
            .map(|part| match part {
                MultimodalContentPart::Text(text) => LiteContentPart::Text { text },
                MultimodalContentPart::ImageUrl { url } => LiteContentPart::ImageUrl {
                    image_url: LiteImageUrl {
                        url,
                        detail: Some("high".to_string()),
                    },
                },
            })
            .collect();
        Some(LiteMessageContent::Parts(litellm_parts))
    } else {
        Some(LiteMessageContent::Text(content))
    }
}

fn tool_message_content_to_litellm(
    content: Option<String>,
    tool_call_id: Option<&str>,
) -> Option<LiteMessageContent> {
    let tool_use_id = normalize_tool_call_id(tool_call_id?)?;
    Some(LiteMessageContent::Parts(vec![
        LiteContentPart::ToolResult {
            tool_use_id: tool_use_id.to_string(),
            content: serde_json::Value::String(content.unwrap_or_default()),
            is_error: None,
        },
    ]))
}

fn to_litellm_role(raw_role: &str) -> Result<LiteMessageRole> {
    match raw_role {
        "system" => Ok(LiteMessageRole::System),
        "user" => Ok(LiteMessageRole::User),
        "assistant" => Ok(LiteMessageRole::Assistant),
        "tool" => Ok(LiteMessageRole::Tool),
        "function" => Ok(LiteMessageRole::Function),
        other => Err(anyhow::anyhow!(
            "unsupported chat role for litellm-rs: {other}"
        )),
    }
}

pub(super) fn chat_message_to_litellm_message(message: ChatMessage) -> Result<LiteChatMessage> {
    let ChatMessage {
        role,
        content,
        tool_calls,
        tool_call_id,
        name,
    } = message;
    let role = to_litellm_role(role.as_str())?;
    let content = if matches!(role, LiteMessageRole::Tool) {
        tool_message_content_to_litellm(content.clone(), tool_call_id.as_deref())
            .or_else(|| chat_message_content_to_litellm(content))
    } else {
        chat_message_content_to_litellm(content)
    };

    Ok(LiteChatMessage {
        role,
        content,
        thinking: None,
        name,
        tool_calls: tool_calls.map(|calls| {
            calls
                .into_iter()
                .map(|call| LiteToolCall {
                    id: call.id,
                    tool_type: call.typ,
                    function: LiteFunctionCall {
                        name: call.function.name,
                        arguments: call.function.arguments,
                    },
                })
                .collect()
        }),
        tool_call_id,
        function_call: None,
    })
}

pub(super) fn content_from_litellm(content: Option<LiteMessageContent>) -> Option<String> {
    match content {
        None => None,
        Some(LiteMessageContent::Text(text)) => Some(text),
        Some(LiteMessageContent::Parts(parts)) => {
            let text = parts
                .into_iter()
                .filter_map(|part| match part {
                    LiteContentPart::Text { text } => Some(text),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("\n");
            if text.is_empty() { None } else { Some(text) }
        }
    }
}

pub(super) fn tool_call_from_litellm(call: LiteToolCall) -> ToolCallOut {
    ToolCallOut {
        id: call.id,
        typ: call.tool_type,
        function: FunctionCall {
            name: call.function.name,
            arguments: call.function.arguments,
        },
    }
}
