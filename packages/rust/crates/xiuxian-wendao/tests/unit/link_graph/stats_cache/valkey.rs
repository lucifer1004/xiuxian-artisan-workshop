use super::redis_client;

#[test]
fn redis_client_opens_trimmed_valid_url() {
    let client = redis_client(" redis://127.0.0.1/ ");
    assert!(client.is_ok());
}

#[test]
fn redis_client_preserves_stats_cache_error_context() {
    let Err(error) = redis_client("  ") else {
        panic!("blank URL should fail");
    };
    assert!(error.contains("link-graph stats cache"));
}
