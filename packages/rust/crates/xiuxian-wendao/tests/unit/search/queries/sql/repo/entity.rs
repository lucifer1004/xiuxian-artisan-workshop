use xiuxian_wendao_runtime::transport::SqlFlightRouteProvider;

use crate::search::queries::sql::provider::StudioSqlFlightRouteProvider;
use crate::search::queries::sql::registration::{
    STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME, STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME,
};
use crate::search::queries::sql::tests::fixtures::{
    fixture_service, nullable_string_column_values, publish_repo_entities, string_column_values,
    u64_column_values,
};
use crate::search::{SearchCorpusKind, SearchPlaneService};

#[tokio::test]
async fn studio_sql_flight_provider_queries_repo_entity_logical_view_across_repos() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    publish_repo_entities(&service, "alpha/repo", "solve", "Shows solve", "rev-1").await;
    publish_repo_entities(&service, "beta/repo", "gamma", "Shows gamma", "rev-2").await;

    let provider = StudioSqlFlightRouteProvider::new(service.clone());
    let logical_view_name = SearchCorpusKind::RepoEntity.to_string();
    let response = provider
        .sql_query_batches(
            format!(
                "SELECT repo_id, entity_kind, name, path FROM {logical_view_name} WHERE entity_kind = 'symbol' ORDER BY repo_id, name"
            )
            .as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("repo entity logical view query batches: {error}"));

    assert_eq!(response.batches.len(), 1);
    assert_eq!(response.batches[0].num_rows(), 2);
    assert_eq!(
        string_column_values(&response.batches[0], "repo_id"),
        vec!["alpha/repo".to_string(), "beta/repo".to_string()]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "entity_kind"),
        vec!["symbol".to_string(), "symbol".to_string()]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "name"),
        vec!["solve".to_string(), "gamma".to_string()]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "path"),
        vec![
            "src/BaseModelica.jl".to_string(),
            "src/BaseModelica.jl".to_string(),
        ]
    );
}

#[tokio::test]
async fn studio_sql_flight_provider_exposes_repo_entity_logical_view_columns_catalog() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    publish_repo_entities(&service, "alpha/repo", "solve", "Shows solve", "rev-1").await;

    let provider = StudioSqlFlightRouteProvider::new(service.clone());
    let logical_view_name = SearchCorpusKind::RepoEntity.to_string();
    let response = provider
        .sql_query_batches(
            format!(
                "SELECT column_name, source_column_name, data_type, ordinal_position, sql_object_kind, column_origin_kind FROM {STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME} WHERE sql_table_name = '{logical_view_name}' ORDER BY ordinal_position LIMIT 5"
            )
            .as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("repo entity columns catalog query batches: {error}"));

    assert_eq!(response.batches.len(), 1);
    assert_eq!(response.batches[0].num_rows(), 5);
    assert_eq!(
        string_column_values(&response.batches[0], "column_name"),
        vec![
            "repo_id".to_string(),
            "id".to_string(),
            "entity_kind".to_string(),
            "name".to_string(),
            "name_folded".to_string(),
        ]
    );
    assert_eq!(
        nullable_string_column_values(&response.batches[0], "source_column_name"),
        vec![
            None,
            Some("id".to_string()),
            Some("entity_kind".to_string()),
            Some("name".to_string()),
            Some("name_folded".to_string()),
        ]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "data_type"),
        vec![
            "Utf8".to_string(),
            "Utf8".to_string(),
            "Utf8".to_string(),
            "Utf8".to_string(),
            "Utf8".to_string(),
        ]
    );
    assert_eq!(
        u64_column_values(&response.batches[0], "ordinal_position"),
        vec![1, 2, 3, 4, 5]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "sql_object_kind"),
        vec![
            "view".to_string(),
            "view".to_string(),
            "view".to_string(),
            "view".to_string(),
            "view".to_string(),
        ]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "column_origin_kind"),
        vec![
            "synthetic".to_string(),
            "projected".to_string(),
            "projected".to_string(),
            "projected".to_string(),
            "projected".to_string(),
        ]
    );
}

#[tokio::test]
async fn studio_sql_flight_provider_exposes_repo_entity_view_sources_catalog() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    let repo_id = "alpha/repo";
    publish_repo_entities(&service, repo_id, "solve", "Shows solve", "rev-1").await;

    let provider = StudioSqlFlightRouteProvider::new(service.clone());
    let logical_view_name = SearchCorpusKind::RepoEntity.to_string();
    let response = provider
        .sql_query_batches(
            format!(
                "SELECT sql_view_name, source_sql_table_name, corpus, repo_id, source_ordinal FROM {STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME} WHERE sql_view_name = '{logical_view_name}'"
            )
            .as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("repo entity view-source query batches: {error}"));

    assert_eq!(response.batches.len(), 1);
    assert_eq!(response.batches[0].num_rows(), 1);
    assert_eq!(
        string_column_values(&response.batches[0], "sql_view_name"),
        vec![logical_view_name]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "source_sql_table_name"),
        vec![SearchPlaneService::repo_entity_table_name(repo_id)]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "corpus"),
        vec![SearchCorpusKind::RepoEntity.to_string()]
    );
    assert_eq!(
        nullable_string_column_values(&response.batches[0], "repo_id"),
        vec![Some(repo_id.to_string())]
    );
    assert_eq!(
        u64_column_values(&response.batches[0], "source_ordinal"),
        vec![1]
    );
}
