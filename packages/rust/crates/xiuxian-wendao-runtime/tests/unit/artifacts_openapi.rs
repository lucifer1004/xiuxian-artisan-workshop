use serde_json::Value;
use xiuxian_wendao_runtime::artifacts::openapi::{
    bundled_wendao_gateway_openapi_document, bundled_wendao_gateway_openapi_path,
    load_bundled_wendao_gateway_openapi_document,
};

#[test]
fn bundled_gateway_openapi_document_text_contains_paths() {
    let document = bundled_wendao_gateway_openapi_document();
    assert!(document.contains("\"paths\""));
    assert!(document.contains("\"Wendao Gateway\""));
}

#[test]
fn bundled_gateway_openapi_document_path_exists() {
    let path = bundled_wendao_gateway_openapi_path();
    assert!(
        path.is_file(),
        "bundled gateway OpenAPI path should exist on disk: {}",
        path.display()
    );
}

#[test]
fn bundled_gateway_openapi_document_loads_as_json() {
    let document = load_bundled_wendao_gateway_openapi_document()
        .unwrap_or_else(|error| panic!("bundled gateway OpenAPI should parse: {error}"));

    assert_eq!(document["openapi"], Value::String("3.1.0".to_string()));
    assert_eq!(
        document["info"]["title"],
        Value::String("Wendao Gateway".to_string())
    );
}
