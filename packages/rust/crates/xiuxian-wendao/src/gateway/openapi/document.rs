//! Bundled `OpenAPI` artifact helpers and invariants for the Wendao gateway.

pub use xiuxian_wendao_runtime::artifacts::openapi::{
    bundled_wendao_gateway_openapi_document, bundled_wendao_gateway_openapi_path,
    load_bundled_wendao_gateway_openapi_document,
};

#[cfg(test)]
#[path = "../../../tests/unit/gateway/openapi/document.rs"]
mod tests;
