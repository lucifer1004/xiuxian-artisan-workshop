use anyhow::Result;
use litellm_rs::core::types::chat::ChatMessage as LiteChatMessage;
use litellm_rs::core::types::content::ContentPart as LiteContentPart;
use litellm_rs::core::types::message::{
    MessageContent as LiteMessageContent, MessageRole as LiteMessageRole,
};
use litellm_rs::core::types::tools::{FunctionCall as LiteFunctionCall, ToolCall as LiteToolCall};

use crate::session::{ChatMessage, FunctionCall, ToolCallOut};

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
    Ok(LiteChatMessage {
        role: to_litellm_role(message.role.as_str())?,
        content: message.content.map(LiteMessageContent::Text),
        thinking: None,
        name: message.name,
        tool_calls: message.tool_calls.map(|calls| {
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
        tool_call_id: message.tool_call_id,
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
