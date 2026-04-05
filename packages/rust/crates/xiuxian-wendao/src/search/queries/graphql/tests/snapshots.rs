use serde_json::json;

use crate::search::queries::graphql::query_graphql_payload_with_context;
use crate::search::queries::tests::snapshots::assert_query_json_snapshot;

use super::fixtures::{fixture_service, graphql_context, publish_reference_hits, sample_hit};

#[tokio::test]
async fn graphql_query_adapter_snapshots_surface_payloads() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    publish_reference_hits(
        &service,
        "graphql-snapshot-1",
        &[
            sample_hit("AlphaService", "src/lib.rs", 10),
            sample_hit("BetaThing", "src/beta.rs", 20),
        ],
    )
    .await;
    let context = graphql_context(service);

    let discovery_document = r#"
    {
      wendao_sql_tables(
        filter: { sql_object_kind: "table" }
        sort: [{ field: "sql_table_name" }]
        limit: 10
      ) {
        sql_table_name
        sql_object_kind
        source_count
      }
    }
    "#;
    let table_query_document = r#"
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
    "#;

    let payload = json!({
        "discovery": graphql_snapshot_payload(&context, discovery_document).await,
        "reference_occurrence": graphql_snapshot_payload(&context, table_query_document).await,
    });

    assert_query_json_snapshot("graphql_query_surface_payload", payload);
}

async fn graphql_snapshot_payload(
    context: &crate::search::queries::graphql::context::GraphqlExecutionContext,
    document: &str,
) -> serde_json::Value {
    let payload = query_graphql_payload_with_context(context, document)
        .await
        .unwrap_or_else(|error| panic!("graphql snapshot document failed: {error}"));
    json!({
        "document": document,
        "payload": payload,
    })
}
