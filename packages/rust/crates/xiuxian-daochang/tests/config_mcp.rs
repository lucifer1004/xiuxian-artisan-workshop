#![allow(
    missing_docs,
    unused_imports,
    dead_code,
    clippy::doc_markdown,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::field_reassign_with_default,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::match_wildcard_for_single_variants,
    clippy::manual_async_fn,
    clippy::manual_assert,
    clippy::too_many_lines,
    clippy::too_many_arguments,
    clippy::unnecessary_literal_bound,
    clippy::needless_pass_by_value,
    clippy::struct_field_names,
    clippy::single_match_else,
    clippy::similar_names,
    clippy::format_collect,
    clippy::async_yields_async,
    clippy::assigning_clones
)]

//! Tests for MCP config loading (mcp.json only, no env fallback).

use omni_agent::load_mcp_config;
use std::io::Write;
use std::path::Path;

fn create_temp_dir() -> tempfile::TempDir {
    match tempfile::tempdir() {
        Ok(dir) => dir,
        Err(error) => panic!("create temp dir: {error}"),
    }
}

fn write_json_file(path: &Path, json: &str) {
    let mut file = match std::fs::File::create(path) {
        Ok(file) => file,
        Err(error) => panic!("create mcp.json: {error}"),
    };
    if let Err(error) = file.write_all(json.as_bytes()) {
        panic!("write mcp.json payload: {error}");
    }
}

#[test]
fn load_mcp_config_missing_file_returns_empty() {
    let dir = create_temp_dir();
    let path = dir.path().join("nonexistent.json");
    let servers = match load_mcp_config(&path) {
        Ok(servers) => servers,
        Err(error) => panic!("load missing config should succeed with empty result: {error}"),
    };
    assert!(servers.is_empty());
}

#[test]
fn load_mcp_config_http_server_preserves_base_url() {
    let dir = create_temp_dir();
    let path = dir.path().join("mcp.json");
    let json = r#"{"mcpServers":{"omniAgent":{"type":"http","url":"http://127.0.0.1:3002"}}}"#;
    write_json_file(&path, json);
    let servers = match load_mcp_config(&path) {
        Ok(servers) => servers,
        Err(error) => panic!("load http mcp config: {error}"),
    };
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].name, "omniAgent");
    assert_eq!(
        servers[0].url.as_deref(),
        Some("http://127.0.0.1:3002"),
        "HTTP URL must be preserved to avoid forcing a legacy MCP route"
    );
    assert!(servers[0].command.is_none());
}

#[test]
fn load_mcp_config_http_server_preserves_existing_sse() {
    let dir = create_temp_dir();
    let path = dir.path().join("mcp.json");
    let json = r#"{"mcpServers":{"omniAgent":{"type":"http","url":"http://127.0.0.1:3002/sse"}}}"#;
    write_json_file(&path, json);
    let servers = match load_mcp_config(&path) {
        Ok(servers) => servers,
        Err(error) => panic!("load http sse mcp config: {error}"),
    };
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].url.as_deref(), Some("http://127.0.0.1:3002/sse"));
}

#[test]
fn load_mcp_config_http_server_trims_messages_trailing_slash() {
    let dir = create_temp_dir();
    let path = dir.path().join("mcp.json");
    let json =
        r#"{"mcpServers":{"omniAgent":{"type":"http","url":"http://127.0.0.1:3002/messages/"}}}"#;
    write_json_file(&path, json);
    let servers = match load_mcp_config(&path) {
        Ok(servers) => servers,
        Err(error) => panic!("load http messages mcp config: {error}"),
    };
    assert_eq!(servers.len(), 1);
    assert_eq!(
        servers[0].url.as_deref(),
        Some("http://127.0.0.1:3002/messages")
    );
}

#[test]
fn load_mcp_config_stdio_server() {
    let dir = create_temp_dir();
    let path = dir.path().join("mcp.json");
    let json = r#"{"mcpServers":{"stdioAgent":{"type":"stdio","command":"omni","args":["mcp","--transport","stdio"]}}}"#;
    write_json_file(&path, json);
    let servers = match load_mcp_config(&path) {
        Ok(servers) => servers,
        Err(error) => panic!("load stdio mcp config: {error}"),
    };
    assert_eq!(servers.len(), 1);
    assert_eq!(servers[0].name, "stdioAgent");
    assert!(servers[0].url.is_none());
    assert_eq!(servers[0].command.as_deref(), Some("omni"));
    assert_eq!(
        servers[0].args.as_deref(),
        Some(
            &[
                "mcp".to_string(),
                "--transport".to_string(),
                "stdio".to_string()
            ][..]
        )
    );
}
