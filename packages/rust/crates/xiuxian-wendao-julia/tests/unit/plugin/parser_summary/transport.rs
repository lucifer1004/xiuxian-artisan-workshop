use xiuxian_wendao_core::repo_intelligence::{RegisteredRepository, RepositoryPluginConfig};

use super::{
    JULIA_FILE_SUMMARY_ROUTE, JULIA_PARSER_SUMMARY_SCHEMA_VERSION, ParserSummaryRouteKind,
    build_julia_parser_summary_flight_transport_client,
};

#[test]
fn build_parser_summary_client_uses_default_discovery_for_plain_plugin_id() {
    let repository = RegisteredRepository {
        id: "repo-julia".to_string(),
        plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
        ..RegisteredRepository::default()
    };

    let client = build_julia_parser_summary_flight_transport_client(
        &repository,
        ParserSummaryRouteKind::FileSummary,
    )
    .unwrap_or_else(|error| panic!("plain Julia plugin id should use default discovery: {error}"));

    assert!(!client.flight_base_url().trim().is_empty());
    assert_eq!(client.flight_route(), JULIA_FILE_SUMMARY_ROUTE);
    assert_eq!(
        client.selection().selected_transport,
        xiuxian_wendao_core::transport::PluginTransportKind::ArrowFlight,
    );
}

#[test]
fn build_parser_summary_client_reads_nested_options() {
    let repository = RegisteredRepository {
        id: "repo-julia".to_string(),
        plugins: vec![RepositoryPluginConfig::Config {
            id: "julia".to_string(),
            options: serde_json::json!({
                "parser_summary_transport": {
                    "base_url": "http://127.0.0.1:9107",
                    "file_summary": {
                        "health_route": "/ready",
                        "timeout_secs": 21
                    }
                }
            }),
        }],
        ..RegisteredRepository::default()
    };

    let client = build_julia_parser_summary_flight_transport_client(
        &repository,
        ParserSummaryRouteKind::FileSummary,
    )
    .unwrap_or_else(|error| panic!("parser-summary config should parse: {error}"));

    assert_eq!(client.flight_base_url(), "http://127.0.0.1:9107");
    assert_eq!(client.flight_route(), JULIA_FILE_SUMMARY_ROUTE);
    assert_eq!(
        client.selection().selected_transport,
        xiuxian_wendao_core::transport::PluginTransportKind::ArrowFlight,
    );
    let _ = JULIA_PARSER_SUMMARY_SCHEMA_VERSION;
}

#[test]
fn build_parser_summary_client_rejects_disabled_transport() {
    let repository = RegisteredRepository {
        id: "repo-julia".to_string(),
        plugins: vec![RepositoryPluginConfig::Config {
            id: "julia".to_string(),
            options: serde_json::json!({
                "parser_summary_transport": {
                    "enabled": false,
                    "base_url": "http://127.0.0.1:9107"
                }
            }),
        }],
        ..RegisteredRepository::default()
    };

    let error = match build_julia_parser_summary_flight_transport_client(
        &repository,
        ParserSummaryRouteKind::FileSummary,
    ) {
        Ok(_) => panic!("disabled parser-summary transport must fail"),
        Err(error) => error,
    };
    assert!(
        error
            .to_string()
            .contains("requires an enabled Julia parser-summary Flight transport client"),
        "unexpected error: {error}",
    );
}

#[test]
fn build_parser_summary_client_rejects_invalid_field_types() {
    let repository = RegisteredRepository {
        id: "repo-julia".to_string(),
        plugins: vec![RepositoryPluginConfig::Config {
            id: "julia".to_string(),
            options: serde_json::json!({
                "parser_summary_transport": {
                    "root_summary": {
                        "timeout_secs": "fast"
                    }
                }
            }),
        }],
        ..RegisteredRepository::default()
    };

    let error = build_julia_parser_summary_flight_transport_client(
        &repository,
        ParserSummaryRouteKind::RootSummary,
    )
    .err()
    .unwrap_or_else(|| panic!("invalid timeout type must fail"));
    assert!(
        error
            .to_string()
            .contains("Julia plugin field `timeout_secs` must be an unsigned integer"),
        "unexpected error: {error}",
    );
}
