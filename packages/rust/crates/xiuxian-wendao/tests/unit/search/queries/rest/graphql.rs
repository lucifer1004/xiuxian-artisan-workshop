use crate::search::queries::rest::{RestQueryPayload, RestQueryRequest, query_rest_payload};

use super::fixtures::{fixture_service, publish_reference_hits, query_service, sample_hit};

#[tokio::test]
async fn rest_graphql_query_routes_through_shared_graphql_adapter() {
    let search_plane_temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&search_plane_temp);
    publish_reference_hits(
        &service,
        "rest-graphql-1",
        &[
            sample_hit("AlphaService", "src/lib.rs", 10),
            sample_hit("BetaThing", "src/beta.rs", 20),
        ],
    )
    .await;
    let query_service = query_service(service);
    let payload = query_rest_payload(
        &query_service,
        &RestQueryRequest::Graphql {
            document: r#"
            {
              reference_occurrence(filter: { name: "AlphaService" }, limit: 1) {
                name
                path
              }
            }
            "#
            .to_string(),
        },
    )
    .await
    .unwrap_or_else(|error| panic!("rest graphql query: {error}"));

    let RestQueryPayload::Graphql(payload) = payload else {
        panic!("expected GraphQL payload from REST query adapter");
    };

    let rows = payload
        .data
        .get("reference_occurrence")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| panic!("rest graphql payload should include reference_occurrence"));
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].get("name"),
        Some(&serde_json::Value::String("AlphaService".to_string()))
    );
}
