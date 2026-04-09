#![cfg(feature = "provider-litellm")]

//! Regression tests for OpenAI `/responses` stream parsing.

use std::collections::HashMap;
use xiuxian_llm::llm::providers::parse_openai_responses_stream;

#[test]
fn responses_stream_parser_remaps_tool_alias_and_collects_output() {
    let raw = r#"data: {"type":"response.output_item.done","item":{"type":"message","content":[{"type":"output_text","text":"hello world"}]}}
data: {"type":"response.output_item.done","item":{"type":"function_call","id":"call_1","call_id":"call_1","name":"qianhuan_reload","arguments":"{\"scope\":\"all\"}"}}
data: [DONE]"#;
    let alias_to_original =
        HashMap::from([("qianhuan_reload".to_string(), "qianhuan.reload".to_string())]);

    let parsed =
        parse_openai_responses_stream(raw, &alias_to_original).expect("responses stream parses");

    assert_eq!(parsed.content.as_deref(), Some("hello world"));
    assert_eq!(parsed.tool_calls.len(), 1);
    assert_eq!(parsed.tool_calls[0].function.name, "qianhuan.reload");
    assert_eq!(
        parsed.tool_calls[0].function.arguments,
        r#"{"scope":"all"}"#
    );
}

#[test]
fn responses_stream_parser_rejects_empty_output() {
    let raw = r#"data: {"type":"response.created"}
data: [DONE]"#;

    let error = parse_openai_responses_stream(raw, &HashMap::new())
        .expect_err("empty stream should return a parsing error");
    assert!(error.to_string().contains("without content or tool calls"));
}

#[test]
fn responses_stream_parser_deduplicates_message_between_done_and_completed_events() {
    let raw = r#"data: {"type":"response.output_item.done","item":{"type":"message","id":"msg_1","content":[{"type":"output_text","text":"Hi there! How can I help today?"}]}}
data: {"type":"response.completed","response":{"output":[{"type":"message","id":"msg_1","content":[{"type":"output_text","text":"Hi there! How can I help today?"}]}]}}
data: [DONE]"#;

    let parsed =
        parse_openai_responses_stream(raw, &HashMap::new()).expect("responses stream parses");
    assert_eq!(
        parsed.content.as_deref(),
        Some("Hi there! How can I help today?")
    );
}

#[test]
fn responses_stream_parser_prefers_message_items_over_output_text_done() {
    let raw = r#"data: {"type":"response.output_text.done","text":"Hi there! How can I help today?"}
data: {"type":"response.output_item.done","item":{"type":"message","id":"msg_1","content":[{"type":"output_text","text":"Hi there! How can I help today?"}]}}
data: [DONE]"#;

    let parsed =
        parse_openai_responses_stream(raw, &HashMap::new()).expect("responses stream parses");
    assert_eq!(
        parsed.content.as_deref(),
        Some("Hi there! How can I help today?")
    );
}
