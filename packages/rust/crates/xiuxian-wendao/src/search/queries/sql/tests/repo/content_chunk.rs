use xiuxian_wendao_runtime::transport::SqlFlightRouteProvider;

use crate::search::queries::sql::provider::StudioSqlFlightRouteProvider;
use crate::search::queries::sql::provider::metadata::StudioSqlFlightMetadata;
use crate::search::queries::sql::registration::{
    STUDIO_SQL_CATALOG_TABLE_NAME, STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME,
};
use crate::search::queries::sql::tests::fixtures::{
    fixture_service, nullable_string_column_values, publish_repo_content_chunks, repo_document,
    string_column_values, u64_column_values,
};
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

#[tokio::test]
async fn studio_sql_flight_provider_queries_repo_content_chunk_with_stable_repo_alias() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    let repo_id = "alpha/repo";
    let documents = vec![
        repo_document("src/lib.rs", "fn alpha() {}\nlet beta = 1;\n", "rust", 10),
        repo_document("README.md", "# alpha\n", "markdown", 20),
    ];
    publish_repo_content_chunks(&service, repo_id, &documents, "rev-1").await;

    let provider = StudioSqlFlightRouteProvider::new(service.clone());
    let sql_table_name = SearchPlaneService::repo_content_chunk_table_name(repo_id);
    let response = provider
        .sql_query_batches(
            format!(
                "SELECT path, line_text FROM {sql_table_name} WHERE path = 'src/lib.rs' ORDER BY line_number"
            )
            .as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("repo SQL query batches: {error}"));

    assert_eq!(response.batches.len(), 1);
    assert_eq!(response.batches[0].num_rows(), 2);
    assert_eq!(
        string_column_values(&response.batches[0], "path"),
        vec!["src/lib.rs".to_string(), "src/lib.rs".to_string()]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "line_text"),
        vec!["fn alpha() {}".to_string(), "let beta = 1;".to_string()]
    );

    let app_metadata: StudioSqlFlightMetadata =
        serde_json::from_slice(response.app_metadata.as_slice())
            .unwrap_or_else(|error| panic!("decode app metadata: {error}"));
    assert_eq!(
        app_metadata.registered_tables,
        vec![
            SearchCorpusKind::RepoContentChunk.to_string(),
            sql_table_name,
            "wendao_sql_columns".to_string(),
            "wendao_sql_tables".to_string(),
            "wendao_sql_view_sources".to_string(),
        ]
    );
    assert_eq!(app_metadata.registered_table_count, 5);
    assert_eq!(app_metadata.registered_view_count, 1);
    assert_eq!(app_metadata.registered_view_source_count, 1);
}

#[tokio::test]
async fn studio_sql_flight_provider_exposes_repo_alias_in_registered_tables_catalog() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    let repo_id = "alpha/repo";
    let documents = vec![repo_document(
        "src/lib.rs",
        "fn alpha() {}\nlet beta = 1;\n",
        "rust",
        10,
    )];
    publish_repo_content_chunks(&service, repo_id, &documents, "rev-1").await;

    let provider = StudioSqlFlightRouteProvider::new(service.clone());
    let record = service
        .repo_corpus_record_for_reads(SearchCorpusKind::RepoContentChunk, repo_id)
        .await
        .unwrap_or_else(|| panic!("repo content record"));
    let publication = record
        .publication
        .unwrap_or_else(|| panic!("repo content publication"));
    let sql_table_name = SearchPlaneService::repo_content_chunk_table_name(repo_id);
    let engine_table_name = SearchPlaneService::repo_publication_engine_table_name(
        SearchCorpusKind::RepoContentChunk,
        publication.publication_id.as_str(),
    );

    let response = provider
        .sql_query_batches(
            format!(
                "SELECT sql_table_name, engine_table_name, corpus, scope, sql_object_kind, source_count, repo_id FROM {STUDIO_SQL_CATALOG_TABLE_NAME} WHERE repo_id = '{repo_id}'"
            )
            .as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("repo catalog query batches: {error}"));

    assert_eq!(response.batches.len(), 1);
    assert_eq!(response.batches[0].num_rows(), 1);
    assert_eq!(
        string_column_values(&response.batches[0], "sql_table_name"),
        vec![sql_table_name]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "engine_table_name"),
        vec![engine_table_name]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "corpus"),
        vec!["repo_content_chunk".to_string()]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "scope"),
        vec!["repo".to_string()]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "sql_object_kind"),
        vec!["table".to_string()]
    );
    assert_eq!(
        u64_column_values(&response.batches[0], "source_count"),
        vec![0]
    );
    assert_eq!(
        nullable_string_column_values(&response.batches[0], "repo_id"),
        vec![Some(repo_id.to_string())]
    );
}

#[tokio::test]
async fn studio_sql_flight_provider_queries_repo_content_chunk_logical_view_across_repos() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    publish_repo_content_chunks(
        &service,
        "alpha/repo",
        &[repo_document(
            "src/lib.rs",
            "fn alpha() {}\nlet beta = 1;\n",
            "rust",
            10,
        )],
        "rev-1",
    )
    .await;
    publish_repo_content_chunks(
        &service,
        "beta/repo",
        &[repo_document("src/lib.rs", "fn gamma() {}\n", "rust", 20)],
        "rev-2",
    )
    .await;

    let provider = StudioSqlFlightRouteProvider::new(service.clone());
    let logical_view_name = SearchCorpusKind::RepoContentChunk.to_string();
    let response = provider
        .sql_query_batches(
            format!(
                "SELECT repo_id, title, doc_type, language_tag, kind_tag, path, line_text FROM {logical_view_name} WHERE path = 'src/lib.rs' ORDER BY repo_id, line_number"
            )
            .as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("repo logical view query batches: {error}"));

    assert_eq!(response.batches.len(), 1);
    assert_eq!(response.batches[0].num_rows(), 3);
    assert_eq!(
        string_column_values(&response.batches[0], "repo_id"),
        vec![
            "alpha/repo".to_string(),
            "alpha/repo".to_string(),
            "beta/repo".to_string(),
        ]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "title"),
        vec![
            "src/lib.rs".to_string(),
            "src/lib.rs".to_string(),
            "src/lib.rs".to_string(),
        ]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "doc_type"),
        vec!["file".to_string(), "file".to_string(), "file".to_string()]
    );
    assert_eq!(
        nullable_string_column_values(&response.batches[0], "language_tag"),
        vec![
            Some("lang:rust".to_string()),
            Some("lang:rust".to_string()),
            Some("lang:rust".to_string()),
        ]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "kind_tag"),
        vec![
            "kind:file".to_string(),
            "kind:file".to_string(),
            "kind:file".to_string(),
        ]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "path"),
        vec![
            "src/lib.rs".to_string(),
            "src/lib.rs".to_string(),
            "src/lib.rs".to_string(),
        ]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "line_text"),
        vec![
            "fn alpha() {}".to_string(),
            "let beta = 1;".to_string(),
            "fn gamma() {}".to_string(),
        ]
    );
}

#[tokio::test]
async fn studio_sql_flight_provider_exposes_repo_logical_view_in_registered_tables_catalog() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    let repo_id = "alpha/repo";
    publish_repo_content_chunks(
        &service,
        repo_id,
        &[repo_document(
            "src/lib.rs",
            "fn alpha() {}\nlet beta = 1;\n",
            "rust",
            10,
        )],
        "rev-1",
    )
    .await;

    let provider = StudioSqlFlightRouteProvider::new(service.clone());
    let logical_view_name = SearchCorpusKind::RepoContentChunk.to_string();
    let response = provider
        .sql_query_batches(
            format!(
                "SELECT sql_table_name, engine_table_name, corpus, scope, sql_object_kind, source_count, repo_id FROM {STUDIO_SQL_CATALOG_TABLE_NAME} WHERE sql_table_name = '{logical_view_name}'"
            )
            .as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("repo logical catalog query batches: {error}"));

    assert_eq!(response.batches.len(), 1);
    assert_eq!(response.batches[0].num_rows(), 1);
    assert_eq!(
        string_column_values(&response.batches[0], "sql_table_name"),
        vec![logical_view_name.clone()]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "engine_table_name"),
        vec![logical_view_name]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "corpus"),
        vec!["repo_content_chunk".to_string()]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "scope"),
        vec!["repo_logical".to_string()]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "sql_object_kind"),
        vec!["view".to_string()]
    );
    assert_eq!(
        u64_column_values(&response.batches[0], "source_count"),
        vec![1]
    );
    assert_eq!(
        nullable_string_column_values(&response.batches[0], "repo_id"),
        vec![None]
    );
}

#[tokio::test]
async fn studio_sql_flight_provider_exposes_repo_content_chunk_logical_columns_catalog() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    publish_repo_content_chunks(
        &service,
        "alpha/repo",
        &[repo_document(
            "src/lib.rs",
            "fn alpha() {}\nlet beta = 1;\n",
            "rust",
            10,
        )],
        "rev-1",
    )
    .await;

    let provider = StudioSqlFlightRouteProvider::new(service.clone());
    let logical_view_name = SearchCorpusKind::RepoContentChunk.to_string();
    let response = provider
        .sql_query_batches(
            format!(
                "SELECT column_name, source_column_name, sql_object_kind, column_origin_kind \
                 FROM wendao_sql_columns \
                 WHERE sql_table_name = '{logical_view_name}' \
                 ORDER BY ordinal_position"
            )
            .as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("repo content chunk logical columns catalog: {error}"));

    assert_eq!(response.batches.len(), 1);
    assert_eq!(response.batches[0].num_rows(), 15);
    assert_eq!(
        string_column_values(&response.batches[0], "column_name"),
        vec![
            "repo_id".to_string(),
            "title".to_string(),
            "doc_type".to_string(),
            "code_tag".to_string(),
            "file_tag".to_string(),
            "kind_tag".to_string(),
            "language_tag".to_string(),
            "id".to_string(),
            "path".to_string(),
            "path_folded".to_string(),
            "language".to_string(),
            "line_number".to_string(),
            "line_text".to_string(),
            "line_text_folded".to_string(),
            "search_text".to_string(),
        ]
    );
    assert_eq!(
        nullable_string_column_values(&response.batches[0], "source_column_name"),
        vec![
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some("id".to_string()),
            Some("path".to_string()),
            Some("path_folded".to_string()),
            Some("language".to_string()),
            Some("line_number".to_string()),
            Some("line_text".to_string()),
            Some("line_text_folded".to_string()),
            Some("search_text".to_string()),
        ]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "sql_object_kind"),
        vec!["view".to_string(); 15]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "column_origin_kind"),
        vec![
            "synthetic".to_string(),
            "synthetic".to_string(),
            "synthetic".to_string(),
            "synthetic".to_string(),
            "synthetic".to_string(),
            "synthetic".to_string(),
            "synthetic".to_string(),
            "projected".to_string(),
            "projected".to_string(),
            "projected".to_string(),
            "projected".to_string(),
            "projected".to_string(),
            "projected".to_string(),
            "projected".to_string(),
            "projected".to_string(),
        ]
    );
}

#[tokio::test]
async fn studio_sql_flight_provider_exposes_repo_logical_view_sources_catalog() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    publish_repo_content_chunks(
        &service,
        "alpha/repo",
        &[repo_document(
            "src/lib.rs",
            "fn alpha() {}\nlet beta = 1;\n",
            "rust",
            10,
        )],
        "rev-1",
    )
    .await;
    publish_repo_content_chunks(
        &service,
        "beta/repo",
        &[repo_document("src/lib.rs", "fn gamma() {}\n", "rust", 20)],
        "rev-2",
    )
    .await;

    let provider = StudioSqlFlightRouteProvider::new(service.clone());
    let logical_view_name = SearchCorpusKind::RepoContentChunk.to_string();
    let response = provider
        .sql_query_batches(
            format!(
                "SELECT sql_view_name, source_sql_table_name, corpus, repo_id, source_ordinal FROM {STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME} WHERE sql_view_name = '{logical_view_name}' ORDER BY source_ordinal"
            )
            .as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("repo logical view-source query batches: {error}"));

    assert_eq!(response.batches.len(), 1);
    assert_eq!(response.batches[0].num_rows(), 2);
    assert_eq!(
        string_column_values(&response.batches[0], "sql_view_name"),
        vec![logical_view_name.clone(), logical_view_name]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "source_sql_table_name"),
        vec![
            SearchPlaneService::repo_content_chunk_table_name("alpha/repo"),
            SearchPlaneService::repo_content_chunk_table_name("beta/repo"),
        ]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "corpus"),
        vec![
            SearchCorpusKind::RepoContentChunk.to_string(),
            SearchCorpusKind::RepoContentChunk.to_string(),
        ]
    );
    assert_eq!(
        nullable_string_column_values(&response.batches[0], "repo_id"),
        vec![
            Some("alpha/repo".to_string()),
            Some("beta/repo".to_string()),
        ]
    );
    assert_eq!(
        u64_column_values(&response.batches[0], "source_ordinal"),
        vec![1, 2]
    );
}

#[tokio::test]
async fn studio_sql_flight_provider_reports_repo_logical_view_fan_in_in_tables_catalog() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let service = fixture_service(&temp_dir);
    publish_repo_content_chunks(
        &service,
        "alpha/repo",
        &[repo_document(
            "src/lib.rs",
            "fn alpha() {}\nlet beta = 1;\n",
            "rust",
            10,
        )],
        "rev-1",
    )
    .await;
    publish_repo_content_chunks(
        &service,
        "beta/repo",
        &[repo_document("src/lib.rs", "fn gamma() {}\n", "rust", 20)],
        "rev-2",
    )
    .await;

    let provider = StudioSqlFlightRouteProvider::new(service.clone());
    let logical_view_name = SearchCorpusKind::RepoContentChunk.to_string();
    let response = provider
        .sql_query_batches(
            format!(
                "SELECT sql_table_name, sql_object_kind, source_count FROM {STUDIO_SQL_CATALOG_TABLE_NAME} WHERE sql_table_name = '{logical_view_name}'"
            )
            .as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("repo logical fan-in catalog query batches: {error}"));

    assert_eq!(response.batches.len(), 1);
    assert_eq!(response.batches[0].num_rows(), 1);
    assert_eq!(
        string_column_values(&response.batches[0], "sql_table_name"),
        vec![logical_view_name]
    );
    assert_eq!(
        string_column_values(&response.batches[0], "sql_object_kind"),
        vec!["view".to_string()]
    );
    assert_eq!(
        u64_column_values(&response.batches[0], "source_count"),
        vec![2]
    );
}
