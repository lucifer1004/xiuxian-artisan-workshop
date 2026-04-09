use std::fs;
use std::sync::Arc;

use arrow::array::{BooleanArray, StringArray, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use tempfile::tempdir;
use xiuxian_wendao_core::{
    repo_intelligence::{
        AnalysisContext, RegisteredRepository, RepoIntelligencePlugin, RepositoryPluginConfig,
        RepositoryRefreshPolicy,
    },
    transport::PluginTransportKind,
};

use super::{
    JULIA_PLUGIN_CAPABILITY_MANIFEST_ROUTE, JULIA_PLUGIN_CAPABILITY_MANIFEST_SCHEMA_VERSION,
    JuliaPluginCapabilityManifestRequestRow, JuliaPluginCapabilityManifestRow,
    build_julia_capability_manifest_flight_transport_client,
    build_julia_plugin_capability_manifest_request_batch,
    decode_julia_plugin_capability_manifest_rows,
    discover_julia_graph_structural_binding_from_manifest_for_repository,
    fetch_julia_plugin_capability_manifest_rows_for_repository,
    graph_structural_binding_from_capability_manifest_rows,
    validate_julia_capability_manifest_preflight_for_repository,
    validate_julia_plugin_capability_manifest_response_batches,
};
use crate::compatibility::link_graph::{
    JULIA_CAPABILITY_MANIFEST_CAPABILITY_ID, JULIA_GRAPH_STRUCTURAL_CAPABILITY_ID, JULIA_PLUGIN_ID,
};
use crate::plugin::entry::JuliaRepoIntelligencePlugin;
use crate::plugin::graph_structural::GraphStructuralRouteKind;
use crate::plugin::graph_structural_transport::build_graph_structural_flight_transport_client;
use crate::plugin::test_support::official_examples::{
    reserve_real_service_port, spawn_real_wendaosearch_demo_capability_manifest_service,
    wait_for_service_ready_with_attempts,
};

fn julia_plugin_capability_manifest_response_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new(
            super::JULIA_PLUGIN_CAPABILITY_MANIFEST_RESPONSE_PLUGIN_ID_COLUMN,
            DataType::Utf8,
            false,
        ),
        Field::new(
            super::JULIA_PLUGIN_CAPABILITY_MANIFEST_CAPABILITY_ID_COLUMN,
            DataType::Utf8,
            false,
        ),
        Field::new(
            super::JULIA_PLUGIN_CAPABILITY_MANIFEST_CAPABILITY_VARIANT_COLUMN,
            DataType::Utf8,
            true,
        ),
        Field::new(
            super::JULIA_PLUGIN_CAPABILITY_MANIFEST_TRANSPORT_KIND_COLUMN,
            DataType::Utf8,
            false,
        ),
        Field::new(
            super::JULIA_PLUGIN_CAPABILITY_MANIFEST_BASE_URL_COLUMN,
            DataType::Utf8,
            false,
        ),
        Field::new(
            super::JULIA_PLUGIN_CAPABILITY_MANIFEST_ROUTE_COLUMN,
            DataType::Utf8,
            false,
        ),
        Field::new(
            super::JULIA_PLUGIN_CAPABILITY_MANIFEST_HEALTH_ROUTE_COLUMN,
            DataType::Utf8,
            true,
        ),
        Field::new(
            super::JULIA_PLUGIN_CAPABILITY_MANIFEST_SCHEMA_VERSION_COLUMN,
            DataType::Utf8,
            false,
        ),
        Field::new(
            super::JULIA_PLUGIN_CAPABILITY_MANIFEST_TIMEOUT_SECS_COLUMN,
            DataType::UInt64,
            true,
        ),
        Field::new(
            super::JULIA_PLUGIN_CAPABILITY_MANIFEST_ENABLED_COLUMN,
            DataType::Boolean,
            false,
        ),
    ]))
}

fn configured_repository(options: serde_json::Value) -> RegisteredRepository {
    RegisteredRepository {
        id: "repo-julia".to_string(),
        plugins: vec![RepositoryPluginConfig::Config {
            id: "julia".to_string(),
            options,
        }],
        ..RegisteredRepository::default()
    }
}

fn live_capability_manifest_repository(base_url: &str) -> RegisteredRepository {
    RegisteredRepository {
        id: "repo-julia".to_string(),
        path: None,
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Config {
            id: "julia".to_string(),
            options: serde_json::json!({
                "capability_manifest_transport": {
                    "base_url": base_url,
                    "route": "/plugin/capabilities",
                    "schema_version": "v0-draft"
                }
            }),
        }],
    }
}

fn sample_response_batch() -> RecordBatch {
    RecordBatch::try_new(
        julia_plugin_capability_manifest_response_schema(),
        vec![
            Arc::new(StringArray::from(vec![
                Some("xiuxian-wendao-julia"),
                Some("xiuxian-wendao-julia"),
            ])),
            Arc::new(StringArray::from(vec![
                Some("rerank"),
                Some("graph-structural"),
            ])),
            Arc::new(StringArray::from(vec![None, Some("structural_rerank")])),
            Arc::new(StringArray::from(vec![
                Some("arrow_flight"),
                Some("arrow_flight"),
            ])),
            Arc::new(StringArray::from(vec![
                Some("http://127.0.0.1:8815"),
                Some("http://127.0.0.1:8816"),
            ])),
            Arc::new(StringArray::from(vec![
                Some("/rerank"),
                Some("/graph/structural/rerank"),
            ])),
            Arc::new(StringArray::from(vec![Some("/healthz"), Some("/ready")])),
            Arc::new(StringArray::from(vec![Some("v1"), Some("v0-draft")])),
            Arc::new(UInt64Array::from(vec![Some(15), None])),
            Arc::new(BooleanArray::from(vec![true, false])),
        ],
    )
    .unwrap_or_else(|error| panic!("sample response batch should build: {error}"))
}

#[test]
fn capability_manifest_build_client_returns_none_without_config() {
    let repository = RegisteredRepository {
        id: "repo-julia".to_string(),
        plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
        ..RegisteredRepository::default()
    };

    let client = build_julia_capability_manifest_flight_transport_client(&repository)
        .unwrap_or_else(|error| panic!("missing config should be ignored: {error}"));
    assert!(client.is_none());
}

#[test]
fn capability_manifest_build_client_reads_nested_options() {
    let repository = configured_repository(serde_json::json!({
        "capability_manifest_transport": {
            "base_url": "http://127.0.0.1:9105",
            "health_route": "/ready",
            "timeout_secs": 21
        }
    }));

    let client = build_julia_capability_manifest_flight_transport_client(&repository)
        .unwrap_or_else(|error| panic!("manifest config should parse: {error}"))
        .unwrap_or_else(|| panic!("manifest client should exist"));

    assert_eq!(client.flight_base_url(), "http://127.0.0.1:9105");
    assert_eq!(
        client.flight_route(),
        JULIA_PLUGIN_CAPABILITY_MANIFEST_ROUTE
    );
    assert_eq!(
        client.selection().selected_transport,
        PluginTransportKind::ArrowFlight
    );
}

#[test]
fn capability_manifest_build_client_rejects_invalid_field_types() {
    let repository = configured_repository(serde_json::json!({
        "capability_manifest_transport": {
            "timeout_secs": "fast"
        }
    }));

    let Err(error) = build_julia_capability_manifest_flight_transport_client(&repository) else {
        panic!("invalid timeout type must fail");
    };
    assert!(
        error
            .to_string()
            .contains("Julia plugin field `timeout_secs` must be an unsigned integer"),
        "unexpected error: {error}"
    );
}

#[test]
fn capability_manifest_request_batch_materializes_rows() {
    let batch = build_julia_plugin_capability_manifest_request_batch(&[
        JuliaPluginCapabilityManifestRequestRow {
            plugin_id: "xiuxian-wendao-julia".to_string(),
            repository_id: "repo-julia".to_string(),
            capability_filter: Some("graph-structural".to_string()),
            include_disabled: true,
        },
    ])
    .unwrap_or_else(|error| panic!("request batch should build: {error}"));

    assert_eq!(batch.num_rows(), 1);
    assert_eq!(batch.schema().fields().len(), 4);
}

#[test]
fn capability_manifest_decode_rows_materializes_bindings_and_variants() {
    let rows = decode_julia_plugin_capability_manifest_rows(&[sample_response_batch()])
        .unwrap_or_else(|error| panic!("response rows should decode: {error}"));

    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[1].capability_variant.as_deref(),
        Some("structural_rerank")
    );

    let binding = rows[0]
        .to_binding()
        .unwrap_or_else(|error| panic!("enabled row should convert into binding: {error}"))
        .unwrap_or_else(|| panic!("enabled row should produce a binding"));
    assert_eq!(binding.selector, rows[0].selector());
    assert_eq!(binding.endpoint.route.as_deref(), Some("/rerank"));
    assert_eq!(binding.contract_version.0, "v1".to_string());

    let disabled_binding = rows[1]
        .to_binding()
        .unwrap_or_else(|error| panic!("disabled row should still validate: {error}"));
    assert!(disabled_binding.is_none());
}

#[test]
fn capability_manifest_response_validation_rejects_unsupported_transport() {
    let batch = RecordBatch::try_new(
        julia_plugin_capability_manifest_response_schema(),
        vec![
            Arc::new(StringArray::from(vec![Some("xiuxian-wendao-julia")])),
            Arc::new(StringArray::from(vec![Some("rerank")])),
            Arc::new(StringArray::from(vec![None::<&str>])),
            Arc::new(StringArray::from(vec![Some("http")])),
            Arc::new(StringArray::from(vec![Some("http://127.0.0.1:8815")])),
            Arc::new(StringArray::from(vec![Some("/rerank")])),
            Arc::new(StringArray::from(vec![Some("/healthz")])),
            Arc::new(StringArray::from(vec![Some(
                JULIA_PLUGIN_CAPABILITY_MANIFEST_SCHEMA_VERSION,
            )])),
            Arc::new(UInt64Array::from(vec![Some(15)])),
            Arc::new(BooleanArray::from(vec![true])),
        ],
    )
    .unwrap_or_else(|error| panic!("invalid transport batch should build: {error}"));

    let Err(error) = validate_julia_plugin_capability_manifest_response_batches(&[batch]) else {
        panic!("unsupported transport should fail");
    };
    assert!(
        error
            .to_string()
            .contains("unsupported `transport_kind` `http`"),
        "unexpected error: {error}"
    );
}

#[test]
fn capability_manifest_selects_graph_structural_binding_by_variant() {
    let rows = vec![
        JuliaPluginCapabilityManifestRow {
            plugin_id: JULIA_PLUGIN_ID.to_string(),
            capability_id: JULIA_GRAPH_STRUCTURAL_CAPABILITY_ID.to_string(),
            capability_variant: Some("structural_rerank".to_string()),
            transport_kind: "arrow_flight".to_string(),
            base_url: "http://127.0.0.1:8815".to_string(),
            route: "/graph/structural/rerank".to_string(),
            health_route: Some("/healthz".to_string()),
            schema_version: "v0-draft".to_string(),
            timeout_secs: Some(15),
            enabled: true,
        },
        JuliaPluginCapabilityManifestRow {
            plugin_id: JULIA_PLUGIN_ID.to_string(),
            capability_id: JULIA_GRAPH_STRUCTURAL_CAPABILITY_ID.to_string(),
            capability_variant: Some("constraint_filter".to_string()),
            transport_kind: "arrow_flight".to_string(),
            base_url: "http://127.0.0.1:8815".to_string(),
            route: "/graph/structural/filter".to_string(),
            health_route: Some("/healthz".to_string()),
            schema_version: "v0-draft".to_string(),
            timeout_secs: Some(15),
            enabled: true,
        },
    ];

    let binding = graph_structural_binding_from_capability_manifest_rows(
        rows.as_slice(),
        GraphStructuralRouteKind::ConstraintFilter,
    )
    .unwrap_or_else(|error| panic!("constraint-filter variant should resolve: {error}"))
    .unwrap_or_else(|| panic!("constraint-filter binding should exist"));

    assert_eq!(
        binding.endpoint.route.as_deref(),
        Some("/graph/structural/filter")
    );
    assert_eq!(binding.contract_version.0, "v0-draft".to_string());
}

#[tokio::test]
#[serial_test::serial(julia_live)]
async fn demo_capability_manifest_live_proof_covers_fetch_preflight_binding_and_plugin_preflight() {
    let port = reserve_real_service_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let _service = spawn_real_wendaosearch_demo_capability_manifest_service(port);
    let repository = live_capability_manifest_repository(&base_url);

    wait_for_service_ready_with_attempts(&format!("http://127.0.0.1:{port}"), 600)
        .await
        .unwrap_or_else(|error| {
            panic!("wait for real WendaoSearch capability-manifest service: {error}")
        });

    let rows = fetch_julia_plugin_capability_manifest_rows_for_repository(
        &repository,
        &[JuliaPluginCapabilityManifestRequestRow {
            plugin_id: JULIA_PLUGIN_ID.to_string(),
            repository_id: repository.id.clone(),
            capability_filter: None,
            include_disabled: true,
        }],
    )
    .await
    .unwrap_or_else(|error| {
        panic!("real WendaoSearch capability-manifest fetch should succeed: {error}")
    });

    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(|row| row.plugin_id == JULIA_PLUGIN_ID));
    assert!(
        rows.iter()
            .any(|row| row.capability_id == JULIA_CAPABILITY_MANIFEST_CAPABILITY_ID)
    );

    let rows = validate_julia_capability_manifest_preflight_for_repository(&repository)
        .unwrap_or_else(|error| {
            panic!("real WendaoSearch capability-manifest preflight should succeed: {error}")
        })
        .unwrap_or_else(|| panic!("manifest transport should be discovered"));

    assert!(
        rows.iter()
            .any(|row| row.capability_id == JULIA_CAPABILITY_MANIFEST_CAPABILITY_ID)
    );

    let binding = discover_julia_graph_structural_binding_from_manifest_for_repository(
        &repository,
        GraphStructuralRouteKind::StructuralRerank,
    )
    .unwrap_or_else(|error| {
        panic!("manifest discovery should derive a graph-structural binding: {error}")
    })
    .unwrap_or_else(|| panic!("graph-structural binding should exist"));

    assert_eq!(
        binding.endpoint.base_url.as_deref(),
        Some(base_url.as_str())
    );
    assert_eq!(
        binding.endpoint.route.as_deref(),
        Some("/graph/structural/rerank")
    );

    let rerank_client = build_graph_structural_flight_transport_client(
        &repository,
        GraphStructuralRouteKind::StructuralRerank,
    )
    .unwrap_or_else(|error| panic!("manifest fallback should parse rerank route: {error}"))
    .unwrap_or_else(|| panic!("manifest fallback rerank client should exist"));
    let filter_client = build_graph_structural_flight_transport_client(
        &repository,
        GraphStructuralRouteKind::ConstraintFilter,
    )
    .unwrap_or_else(|error| panic!("manifest fallback should parse filter route: {error}"))
    .unwrap_or_else(|| panic!("manifest fallback filter client should exist"));

    assert_eq!(rerank_client.flight_base_url(), base_url);
    assert_eq!(rerank_client.flight_route(), "/graph/structural/rerank");
    assert_eq!(filter_client.flight_base_url(), base_url);
    assert_eq!(filter_client.flight_route(), "/graph/structural/filter");

    let temp = tempdir().unwrap_or_else(|error| panic!("create temp repo: {error}"));
    fs::create_dir_all(temp.path().join("src"))
        .unwrap_or_else(|error| panic!("create src directory: {error}"));
    fs::write(
        temp.path().join("Project.toml"),
        "name = \"DemoPkg\"\nversion = \"0.1.0\"\n",
    )
    .unwrap_or_else(|error| panic!("write Project.toml: {error}"));
    fs::write(
        temp.path().join("src").join("DemoPkg.jl"),
        "module DemoPkg\n\nexport greet\n\ngreet() = :ok\n\nend\n",
    )
    .unwrap_or_else(|error| panic!("write root Julia module: {error}"));

    let repository_with_path = RegisteredRepository {
        path: Some(temp.path().to_path_buf()),
        ..repository.clone()
    };
    let context = AnalysisContext {
        repository: repository_with_path,
        repository_root: temp.path().to_path_buf(),
    };

    JuliaRepoIntelligencePlugin
        .preflight_repository(&context, temp.path())
        .unwrap_or_else(|error| {
            panic!("repository preflight with live capability manifest should succeed: {error}")
        });
}
