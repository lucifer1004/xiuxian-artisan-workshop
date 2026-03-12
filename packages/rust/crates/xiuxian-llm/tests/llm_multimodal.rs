//! Integration tests for platform-neutral multimodal marker parsing.

use anyhow::Result;
use axum::{Router, http::StatusCode, routing::get};
use xiuxian_llm::llm::multimodal::{
    MultimodalContentPart, parse_multimodal_text_content, resolve_image_source_to_base64,
};

fn require_some<T>(value: Option<T>, context: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("{context}"),
    }
}

#[test]
fn multimodal_parser_extracts_http_image_marker() {
    let parts = require_some(
        parse_multimodal_text_content("analyze [IMAGE:https://example.com/a.png] now"),
        "expected parsed multimodal parts",
    );
    assert_eq!(
        parts,
        vec![
            MultimodalContentPart::Text("analyze ".to_string()),
            MultimodalContentPart::ImageUrl {
                url: "https://example.com/a.png".to_string()
            },
            MultimodalContentPart::Text(" now".to_string()),
        ]
    );
}

#[test]
fn multimodal_parser_extracts_data_uri_image_marker() {
    let parts = require_some(
        parse_multimodal_text_content("look [PHOTO:data:image/png;base64,AAEC]"),
        "expected parsed multimodal parts",
    );
    assert_eq!(
        parts,
        vec![
            MultimodalContentPart::Text("look ".to_string()),
            MultimodalContentPart::ImageUrl {
                url: "data:image/png;base64,AAEC".to_string()
            },
        ]
    );
}

#[test]
fn multimodal_parser_ignores_invalid_marker() {
    assert!(parse_multimodal_text_content("look [IMAGE:not-a-url]").is_none());
    assert!(parse_multimodal_text_content("look [VIDEO:https://example.com/v.mp4]").is_none());
}

async fn image_bytes_handler() -> (StatusCode, [(&'static str, &'static str); 1], Vec<u8>) {
    (
        StatusCode::OK,
        [("content-type", "image/png")],
        vec![1_u8, 2_u8, 3_u8],
    )
}

async fn spawn_image_server() -> Result<(String, tokio::task::JoinHandle<()>)> {
    let app = Router::new().route("/img.png", get(image_bytes_handler));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Ok((format!("http://{addr}"), handle))
}

#[tokio::test]
async fn multimodal_resolver_accepts_data_uri_without_network_fetch() -> Result<()> {
    let source =
        resolve_image_source_to_base64(&reqwest::Client::new(), "data:image/jpeg;base64,AAEC")
            .await?;
    assert_eq!(source.media_type, "image/jpeg");
    assert_eq!(source.data, "AAEC");
    Ok(())
}

#[tokio::test]
async fn multimodal_resolver_fetches_http_image_and_encodes_base64() -> Result<()> {
    let (base, server_handle) = spawn_image_server().await?;
    let url = format!("{base}/img.png");
    let source = resolve_image_source_to_base64(&reqwest::Client::new(), url.as_str()).await?;
    assert_eq!(source.media_type, "image/png");
    assert_eq!(source.data, "AQID");
    server_handle.abort();
    Ok(())
}
