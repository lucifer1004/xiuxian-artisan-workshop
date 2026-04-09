use anyhow::Result;

use super::{ToolRuntimeCallWireResult, ToolRuntimeJsonRpcResponse, try_parse_sse_message};

#[test]
fn sse_parser_skips_retry_priming_event() -> Result<()> {
    let payload =
        "id: 0\nretry: 3000\n\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{\"tools\":[]}}\n\n";
    let parsed = try_parse_sse_message(payload)?
        .ok_or_else(|| anyhow::anyhow!("json-rpc message should be extracted"))?;
    let response: ToolRuntimeJsonRpcResponse<serde_json::Value> = serde_json::from_value(parsed)?;
    assert_eq!(response.jsonrpc, "2.0");
    Ok(())
}

#[test]
fn call_result_deserialization_preserves_text_and_error_flag() -> Result<()> {
    let payload = serde_json::json!({
        "content": [
            { "type": "text", "text": "hello" },
            { "type": "image", "data": "ignored", "mimeType": "image/png" }
        ],
        "isError": true
    });
    let result: ToolRuntimeCallWireResult = serde_json::from_value(payload)?;
    assert_eq!(result.content.len(), 2);
    assert_eq!(result.content[0].text.as_deref(), Some("hello"));
    assert_eq!(result.is_error, Some(true));
    Ok(())
}
