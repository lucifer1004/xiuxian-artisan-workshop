#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::implicit_clone,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::field_reassign_with_default,
    clippy::manual_async_fn,
    clippy::async_yields_async,
    clippy::no_effect_underscore_binding
)]

use xiuxian_llm::mcp::{McpPoolConnectConfig, connect_pool};

#[tokio::test]
async fn connect_pool_rejects_zero_pool_size() {
    let cfg = McpPoolConnectConfig {
        pool_size: 0,
        ..McpPoolConnectConfig::default()
    };

    let result = connect_pool("http://127.0.0.1:65535/mcp", cfg, None).await;
    assert!(result.is_err());
    let message = format!("{:?}", result.err());
    assert!(message.contains("pool_size"));
}
