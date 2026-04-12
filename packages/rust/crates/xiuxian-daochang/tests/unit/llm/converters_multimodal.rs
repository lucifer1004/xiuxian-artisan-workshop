use litellm_rs::core::types::content::ContentPart as LiteContentPart;
use litellm_rs::core::types::message::MessageContent as LiteMessageContent;
use litellm_rs::core::types::message::MessageRole as LiteMessageRole;

use xiuxian_daochang::ChatMessage;
use xiuxian_daochang::test_support::{
    chat_message_to_litellm_message, chat_message_to_litellm_message_for_openai_chat,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn build_user_message(content: &str) -> ChatMessage {
    ChatMessage {
        role: "user".to_string(),
        content: Some(content.to_string()),
        tool_calls: None,
        tool_call_id: None,
        name: None,
    }
}

#[test]
fn litellm_converter_turns_image_marker_into_multimodal_parts() -> TestResult {
    let message = build_user_message("please inspect [IMAGE:https://example.com/photo.png]");
    let converted = chat_message_to_litellm_message(message)?;
    let Some(content) = converted.content else {
        panic!("content should exist");
    };
    match content {
        LiteMessageContent::Parts(parts) => {
            assert_eq!(parts.len(), 2);
            match &parts[0] {
                LiteContentPart::Text { text } => assert_eq!(text, "please inspect "),
                other => panic!("expected text part, got {other:?}"),
            }
            match &parts[1] {
                LiteContentPart::ImageUrl { image_url } => {
                    assert_eq!(image_url.url, "https://example.com/photo.png");
                    assert_eq!(image_url.detail.as_deref(), Some("high"));
                }
                other => panic!("expected image_url part, got {other:?}"),
            }
        }
        LiteMessageContent::Text(text) => {
            panic!("expected multipart content, got text: {text:?}")
        }
    }
    Ok(())
}

#[test]
fn litellm_converter_accepts_data_url_image_marker() -> TestResult {
    let message = build_user_message("look [IMAGE:data:image/jpeg;base64,AAEC]");
    let converted = chat_message_to_litellm_message(message)?;
    let Some(content) = converted.content else {
        panic!("content should exist");
    };
    match content {
        LiteMessageContent::Parts(parts) => {
            assert_eq!(parts.len(), 2);
            match &parts[1] {
                LiteContentPart::ImageUrl { image_url } => {
                    assert_eq!(image_url.url, "data:image/jpeg;base64,AAEC");
                    assert_eq!(image_url.detail.as_deref(), Some("high"));
                }
                other => panic!("expected image_url part, got {other:?}"),
            }
        }
        LiteMessageContent::Text(text) => {
            panic!("expected multipart content, got text: {text:?}")
        }
    }
    Ok(())
}

#[test]
fn litellm_converter_keeps_invalid_image_marker_as_plain_text() -> TestResult {
    let raw = "look [IMAGE:not-a-url]";
    let message = build_user_message(raw);
    let converted = chat_message_to_litellm_message(message)?;
    let Some(content) = converted.content else {
        panic!("content should exist");
    };
    match content {
        LiteMessageContent::Text(text) => assert_eq!(text, raw),
        LiteMessageContent::Parts(parts) => {
            panic!("expected plain text content, got multipart: {parts:?}")
        }
    }
    Ok(())
}

#[test]
fn litellm_converter_maps_tool_message_to_tool_result_part() -> TestResult {
    let message = ChatMessage {
        role: "tool".to_string(),
        content: Some("{\"ok\":true}".to_string()),
        tool_calls: None,
        tool_call_id: Some("call_123".to_string()),
        name: Some("web.crawl".to_string()),
    };
    let converted = chat_message_to_litellm_message(message)?;
    assert!(matches!(converted.role, LiteMessageRole::Tool));
    let Some(content) = converted.content else {
        panic!("content should exist");
    };
    match content {
        LiteMessageContent::Parts(parts) => {
            assert_eq!(parts.len(), 1);
            match &parts[0] {
                LiteContentPart::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                } => {
                    assert_eq!(tool_use_id, "call_123");
                    assert_eq!(
                        content,
                        &serde_json::Value::String("{\"ok\":true}".to_string())
                    );
                    assert_eq!(*is_error, None);
                }
                other => panic!("expected tool_result part, got {other:?}"),
            }
        }
        LiteMessageContent::Text(text) => {
            panic!("expected multipart content, got text: {text:?}")
        }
    }
    Ok(())
}

#[test]
fn openai_chat_converter_keeps_tool_message_as_text_content() -> TestResult {
    let message = ChatMessage {
        role: "tool".to_string(),
        content: Some("{\"ok\":true}".to_string()),
        tool_calls: None,
        tool_call_id: Some("call_123".to_string()),
        name: Some("web.crawl".to_string()),
    };
    let converted = chat_message_to_litellm_message_for_openai_chat(message)?;
    assert!(matches!(converted.role, LiteMessageRole::Tool));
    match converted.content {
        Some(LiteMessageContent::Text(text)) => assert_eq!(text, "{\"ok\":true}"),
        other => panic!("expected plain text content for openai chat tool message, got {other:?}"),
    }
    assert_eq!(converted.tool_call_id.as_deref(), Some("call_123"));
    Ok(())
}

#[test]
fn litellm_converter_accepts_developer_role() -> TestResult {
    let message = ChatMessage {
        role: "developer".to_string(),
        content: Some("Prefer concise answers.".to_string()),
        tool_calls: None,
        tool_call_id: None,
        name: None,
    };
    let converted = chat_message_to_litellm_message(message)?;
    assert!(matches!(converted.role, LiteMessageRole::Developer));
    match converted.content {
        Some(LiteMessageContent::Text(text)) => assert_eq!(text, "Prefer concise answers."),
        other => panic!("expected developer text content, got {other:?}"),
    }
    Ok(())
}
