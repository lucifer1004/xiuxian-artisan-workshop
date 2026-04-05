use crate::search::queries::graphql::query_graphql_payload_with_context;
use crate::search::queries::graphql::tests::fixtures::{
    fixture_service, graphql_context, publish_reference_hits, sample_hit,
};

#[tokio::test]
async fn graphql_sql_tables_field_routes_through_shared_sql_discovery_surface() {
    let search_plane_temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&search_plane_temp);
    publish_reference_hits(
        &service,
        "graphql-discovery-1",
        &[sample_hit("AlphaService", "src/lib.rs", 10)],
    )
    .await;
    let context = graphql_context(service);

    let payload = query_graphql_payload_with_context(
        &context,
        r#"
        {
          wendao_sql_tables(
            filter: { sql_object_kind: "table" }
            sort: [{ field: "sql_table_name" }]
            limit: 10
          ) {
            sql_table_name
            sql_object_kind
            source_count
            repo_id
          }
        }
        "#,
    )
    .await
    .unwrap_or_else(|error| panic!("graphql wendao_sql_tables query: {error}"));

    let tables = payload
        .data
        .get("wendao_sql_tables")
        .and_then(serde_json::Value::as_array)
        .unwrap_or_else(|| panic!("graphql payload should include wendao_sql_tables array"));
    assert!(
        tables.iter().any(|row| {
            row.get("sql_table_name")
                == Some(&serde_json::Value::String(
                    "reference_occurrence".to_string(),
                ))
        }),
        "graphql discovery should expose reference_occurrence"
    );
}

#[tokio::test]
async fn graphql_rejects_unsupported_root_field() {
    let search_plane_temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&search_plane_temp);
    let context = graphql_context(service);

    let error = query_graphql_payload_with_context(
        &context,
        "query { unsupportedRoot(unknown: 1) { value } another { field } }",
    )
    .await;

    let Err(error) = error else {
        panic!("expected unsupported root field to fail");
    };

    assert!(error.contains("exactly one root field"));
}
