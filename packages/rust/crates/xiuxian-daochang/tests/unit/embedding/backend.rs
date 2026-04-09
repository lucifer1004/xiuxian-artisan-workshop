use xiuxian_daochang::test_support::{EmbeddingBackendMode, parse_embedding_client_backend_mode};

#[test]
fn parse_backend_mode_supports_openai_aliases() {
    assert_eq!(
        parse_embedding_client_backend_mode(Some("openai_http")),
        EmbeddingBackendMode::OpenAiHttp
    );
}

#[test]
fn parse_backend_mode_retains_legacy_http_alias() {
    assert_eq!(
        parse_embedding_client_backend_mode(Some("http")),
        EmbeddingBackendMode::Http
    );
    assert_eq!(
        parse_embedding_client_backend_mode(Some("client")),
        EmbeddingBackendMode::Http
    );
}
