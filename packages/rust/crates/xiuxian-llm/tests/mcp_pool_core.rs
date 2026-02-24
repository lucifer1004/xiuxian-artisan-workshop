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

use xiuxian_llm::mcp::{McpPoolConnectConfig, connect_pool_clients_with_retry};

#[tokio::test]
async fn connect_pool_clients_with_retry_rejects_zero_pool_size() {
    let mut cfg = McpPoolConnectConfig::default();
    cfg.pool_size = 0;

    let result = connect_pool_clients_with_retry("http://127.0.0.1:65535/mcp", cfg).await;

    match result {
        Ok(_) => panic!("pool size 0 should be rejected before connect"),
        Err(error) => assert!(
            error
                .to_string()
                .contains("pool_size must be greater than 0"),
            "unexpected error: {error}"
        ),
    }
}
