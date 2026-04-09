use crate::search::queries::graphql::context::GraphqlExecutionContext;
use crate::search::queries::graphql::query_graphql_payload_with_context;
use crate::search::queries::graphql::tests::fixtures::{
    fixture_service, graphql_context, publish_reference_hits, sample_hit,
};

#[tokio::test]
async fn graphql_table_query_routes_through_request_scoped_reference_occurrence_table() {
    let search_plane_temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&search_plane_temp);
    publish_reference_hits(
        &service,
        "graphql-table-query-1",
        &[
            sample_hit("AlphaService", "src/lib.rs", 10),
            sample_hit("BetaThing", "src/beta.rs", 20),
        ],
    )
    .await;
    let context = graphql_context(service);

    let payload = query_graphql_payload_with_context(
        &context,
        r#"
        {
          reference_occurrence(
            filter: { name: "AlphaService" }
            sort: [{ field: "line", order: "desc" }]
            limit: 1
          ) {
            name
            path
            line
          }
        }
        "#,
    )
    .await
    .unwrap_or_else(|error| panic!("graphql reference_occurrence query: {error}"));

    let rows = payload
        .data
        .get("reference_occurrence")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| panic!("graphql payload should include reference_occurrence array"));
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].get("name"),
        Some(&serde_json::Value::String("AlphaService".to_string()))
    );
    assert_eq!(
        rows[0].get("path"),
        Some(&serde_json::Value::String("src/lib.rs".to_string()))
    );
    assert_eq!(
        rows[0].get("line"),
        Some(&serde_json::Value::Number(serde_json::Number::from(10_u64)))
    );
}

#[tokio::test]
async fn graphql_table_query_requires_shared_query_service_context() {
    let context = GraphqlExecutionContext::new();
    let error = query_graphql_payload_with_context(
        &context,
        r"
        {
          wendao_sql_tables(limit: 1) {
            sql_table_name
          }
        }
        ",
    )
    .await;

    let Err(error) = error else {
        panic!("expected GraphQL query without shared query service to fail");
    };

    assert!(error.contains("require a shared query service"));
}
