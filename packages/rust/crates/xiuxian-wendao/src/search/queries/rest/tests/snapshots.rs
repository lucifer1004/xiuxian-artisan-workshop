use serde_json::json;

use crate::search::queries::rest::{RestQueryRequest, query_rest_payload};
use crate::search::queries::tests::snapshots::assert_query_json_snapshot;

use super::fixtures::{fixture_service, publish_reference_hits, query_service, sample_hit};

#[tokio::test]
async fn rest_query_adapter_snapshots_surface_payloads() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    publish_reference_hits(
        &service,
        "rest-snapshot-1",
        &[
            sample_hit("AlphaService", "src/lib.rs", 10),
            sample_hit("BetaThing", "src/beta.rs", 20),
        ],
    )
    .await;
    let query_service = query_service(service);
    let sql_request = RestQueryRequest::Sql {
        query: "SELECT name, path FROM reference_occurrence ORDER BY name LIMIT 1".to_string(),
    };
    let graphql_request = RestQueryRequest::Graphql {
        document: r#"
        {
          reference_occurrence(
            filter: { name: "AlphaService" }
            limit: 1
          ) {
            name
            path
          }
        }
        "#
        .to_string(),
    };

    let payload = json!({
        "sql": rest_snapshot_payload(&query_service, &sql_request).await,
        "graphql": rest_snapshot_payload(&query_service, &graphql_request).await,
    });

    assert_query_json_snapshot("rest_query_surface_payload", payload);
}

async fn rest_snapshot_payload(
    service: &crate::search::queries::SearchQueryService,
    request: &RestQueryRequest,
) -> serde_json::Value {
    let payload = query_rest_payload(service, request)
        .await
        .unwrap_or_else(|error| panic!("rest snapshot request failed: {error}"));
    json!({
        "request": request,
        "payload": payload,
    })
}
