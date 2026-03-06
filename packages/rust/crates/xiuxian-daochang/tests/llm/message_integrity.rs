use xiuxian_daochang::test_support::enforce_tool_message_integrity;
use xiuxian_daochang::{ChatMessage, FunctionCall, ToolCallOut};

fn user(content: &str) -> ChatMessage {
    ChatMessage {
        role: "user".to_string(),
        content: Some(content.to_string()),
        tool_calls: None,
        tool_call_id: None,
        name: None,
    }
}

fn assistant_with_tool_calls(ids: &[&str]) -> ChatMessage {
    let calls = ids
        .iter()
        .map(|id| ToolCallOut {
            id: (*id).to_string(),
            typ: "function".to_string(),
            function: FunctionCall {
                name: "web.crawl".to_string(),
                arguments: "{}".to_string(),
            },
        })
        .collect::<Vec<_>>();
    ChatMessage {
        role: "assistant".to_string(),
        content: None,
        tool_calls: Some(calls),
        tool_call_id: None,
        name: None,
    }
}

fn assistant(content: &str) -> ChatMessage {
    ChatMessage {
        role: "assistant".to_string(),
        content: Some(content.to_string()),
        tool_calls: None,
        tool_call_id: None,
        name: None,
    }
}

fn tool(tool_call_id: &str) -> ChatMessage {
    ChatMessage {
        role: "tool".to_string(),
        content: Some("{\"ok\":true}".to_string()),
        tool_calls: None,
        tool_call_id: Some(tool_call_id.to_string()),
        name: Some("web.crawl".to_string()),
    }
}

#[test]
fn tool_message_integrity_keeps_valid_chain() {
    let messages = vec![
        user("fetch"),
        assistant_with_tool_calls(&["call-1"]),
        tool("call-1"),
        assistant("done"),
    ];

    let (sanitized, report) = enforce_tool_message_integrity(messages.clone());
    assert_eq!(sanitized.len(), messages.len());
    assert_eq!(report.dropped_total(), 0);
}

#[test]
fn tool_message_integrity_normalizes_pipe_delimited_ids() {
    let messages = vec![
        user("fetch"),
        assistant_with_tool_calls(&["call-1|fc-1"]),
        tool("call-1|tool-result-shadow"),
        assistant("done"),
    ];

    let (sanitized, report) = enforce_tool_message_integrity(messages.clone());
    assert_eq!(sanitized.len(), messages.len());
    assert_eq!(report.dropped_total(), 0);
}

#[test]
fn tool_message_integrity_drops_incomplete_assistant_and_linked_tools() {
    let messages = vec![
        user("task"),
        assistant_with_tool_calls(&["call-1", "call-2"]),
        tool("call-1"),
        user("newest question"),
    ];

    let (sanitized, report) = enforce_tool_message_integrity(messages);
    assert_eq!(sanitized.len(), 2);
    assert_eq!(sanitized[0].role, "user");
    assert_eq!(sanitized[1].content.as_deref(), Some("newest question"));
    assert_eq!(report.incomplete_assistants, 1);
    assert_eq!(report.linked_tools, 1);
}

#[test]
fn tool_message_integrity_drops_chain_when_non_tool_interrupts_tool_block() {
    let messages = vec![
        user("task"),
        assistant_with_tool_calls(&["call-1"]),
        user("interrupt before tool result"),
        tool("call-1"),
        assistant("fallback"),
    ];

    let (sanitized, report) = enforce_tool_message_integrity(messages);
    assert_eq!(sanitized.len(), 3);
    assert_eq!(sanitized[0].role, "user");
    assert_eq!(
        sanitized[1].content.as_deref(),
        Some("interrupt before tool result")
    );
    assert_eq!(sanitized[2].content.as_deref(), Some("fallback"));
    assert_eq!(report.incomplete_assistants, 1);
    assert_eq!(report.orphan_tools, 1);
}

#[test]
fn tool_message_integrity_drops_orphan_tool_messages() {
    let messages = vec![user("hello"), tool("missing-call"), assistant("ok")];

    let (sanitized, report) = enforce_tool_message_integrity(messages);
    assert_eq!(sanitized.len(), 2);
    assert!(sanitized.iter().all(|msg| msg.role != "tool"));
    assert_eq!(report.orphan_tools, 1);
}

#[test]
fn tool_message_integrity_drops_empty_tool_call_assistant_message() {
    let messages = vec![assistant_with_tool_calls(&[""]), user("still here")];

    let (sanitized, report) = enforce_tool_message_integrity(messages);
    assert_eq!(sanitized.len(), 1);
    assert_eq!(sanitized[0].role, "user");
    assert_eq!(report.empty_tool_call_assistants, 1);
}
