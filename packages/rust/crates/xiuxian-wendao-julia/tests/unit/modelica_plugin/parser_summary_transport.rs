use xiuxian_wendao_core::repo_intelligence::{RegisteredRepository, RepositoryPluginConfig};

use super::{
    MODELICA_FILE_SUMMARY_ROUTE, ParserSummaryRouteKind,
    build_modelica_parser_summary_flight_transport_client,
    modelica_parser_summary_transport_cache_len_for_tests,
    modelica_parser_summary_transport_slot_id_for_tests,
};

fn parser_summary_repository() -> RegisteredRepository {
    RegisteredRepository {
        id: "repo-modelica".to_string(),
        plugins: vec![RepositoryPluginConfig::Id("modelica".to_string())],
        ..RegisteredRepository::default()
    }
}

#[test]
#[serial_test::serial(modelica_parser_summary_transport)]
fn build_modelica_parser_summary_client_uses_default_discovery_for_plain_plugin_id() {
    let repository = parser_summary_repository();

    let client = build_modelica_parser_summary_flight_transport_client(
        &repository,
        ParserSummaryRouteKind::FileSummary,
    )
    .unwrap_or_else(|error| {
        panic!("plain Modelica plugin id should use default discovery: {error}")
    });

    assert!(!client.flight_base_url().trim().is_empty());
    assert_eq!(client.flight_route(), MODELICA_FILE_SUMMARY_ROUTE);
    assert_eq!(
        client.selection().selected_transport,
        xiuxian_wendao_core::transport::PluginTransportKind::ArrowFlight,
    );
}

#[test]
#[serial_test::serial(modelica_parser_summary_transport)]
fn build_modelica_parser_summary_client_reuses_cached_transport_slot_for_same_binding() {
    let repository = parser_summary_repository();
    let baseline = modelica_parser_summary_transport_cache_len_for_tests();

    let _client_a = build_modelica_parser_summary_flight_transport_client(
        &repository,
        ParserSummaryRouteKind::FileSummary,
    )
    .unwrap_or_else(|error| {
        panic!("first Modelica parser-summary client build should succeed: {error}")
    });
    let slot_a = modelica_parser_summary_transport_slot_id_for_tests(
        &repository,
        ParserSummaryRouteKind::FileSummary,
    )
    .unwrap_or_else(|error| panic!("expected cached slot after first build: {error}"));

    let _client_b = build_modelica_parser_summary_flight_transport_client(
        &repository,
        ParserSummaryRouteKind::FileSummary,
    )
    .unwrap_or_else(|error| {
        panic!("second Modelica parser-summary client build should succeed: {error}")
    });
    let slot_b = modelica_parser_summary_transport_slot_id_for_tests(
        &repository,
        ParserSummaryRouteKind::FileSummary,
    )
    .unwrap_or_else(|error| panic!("expected cached slot after second build: {error}"));

    assert_eq!(slot_a, slot_b);
    assert_eq!(
        modelica_parser_summary_transport_cache_len_for_tests(),
        baseline + 1
    );
}
