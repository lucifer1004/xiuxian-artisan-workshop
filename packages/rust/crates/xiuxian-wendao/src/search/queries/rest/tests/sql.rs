use crate::search::queries::rest::{RestQueryPayload, RestQueryRequest, query_rest_payload};

use super::fixtures::{fixture_service, publish_reference_hits, query_service, sample_hit};

#[tokio::test]
async fn rest_sql_query_routes_through_shared_sql_adapter() {
    let search_plane_temp = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&search_plane_temp);
    publish_reference_hits(
        &service,
        "rest-sql-1",
        &[
            sample_hit("AlphaService", "src/lib.rs", 10),
            sample_hit("BetaThing", "src/beta.rs", 20),
        ],
    )
    .await;
    let query_service = query_service(service);
    let payload = query_rest_payload(
        &query_service,
        &RestQueryRequest::Sql {
            query: "SELECT name, path FROM reference_occurrence WHERE name = 'AlphaService'"
                .to_string(),
        },
    )
    .await
    .unwrap_or_else(|error| panic!("rest sql query: {error}"));

    let RestQueryPayload::Sql(payload) = payload else {
        panic!("expected SQL payload from REST query adapter");
    };

    assert_eq!(payload.metadata.result_row_count, 1);
    assert_eq!(
        payload.batches[0].rows[0]
            .get("name")
            .unwrap_or_else(|| panic!("rest sql payload should include `name`")),
        "AlphaService"
    );
}
