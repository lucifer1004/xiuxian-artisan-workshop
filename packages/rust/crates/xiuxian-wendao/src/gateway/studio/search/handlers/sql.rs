use std::collections::BTreeSet;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use xiuxian_vector::engine_batches_to_lance_batches;
use xiuxian_wendao_runtime::transport::{SqlFlightRouteProvider, SqlFlightRouteResponse};

use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

#[derive(Clone)]
pub(crate) struct StudioSqlFlightRouteProvider {
    service: SearchPlaneService,
}

impl StudioSqlFlightRouteProvider {
    #[must_use]
    pub(crate) fn new(service: SearchPlaneService) -> Self {
        Self { service }
    }

    async fn register_readable_tables(&self) -> Result<Vec<String>, String> {
        let mut table_names = BTreeSet::new();
        self.register_local_tables(&mut table_names).await?;
        self.register_repo_tables(&mut table_names).await?;
        Ok(table_names.into_iter().collect())
    }

    async fn register_local_tables(
        &self,
        table_names: &mut BTreeSet<String>,
    ) -> Result<(), String> {
        for corpus in SearchCorpusKind::ALL
            .into_iter()
            .filter(|corpus| !corpus.is_repo_backed())
        {
            let status = self.service.coordinator().status_for(corpus);
            let Some(active_epoch) = status.active_epoch else {
                continue;
            };

            if corpus == SearchCorpusKind::LocalSymbol {
                for table_name in self
                    .service
                    .local_epoch_table_names_for_reads(corpus, active_epoch)
                {
                    let parquet_path = self
                        .service
                        .local_table_parquet_path(corpus, table_name.as_str());
                    if !parquet_path.exists() {
                        continue;
                    }
                    self.service
                        .search_engine()
                        .ensure_parquet_table_registered(
                            table_name.as_str(),
                            parquet_path.as_path(),
                            &[],
                        )
                        .await
                        .map_err(|error| {
                            format!(
                                "studio SQL Flight provider failed to register `{table_name}` for corpus `{corpus}`: {error}"
                            )
                        })?;
                    table_names.insert(table_name);
                }
                continue;
            }

            let parquet_path = self.service.local_epoch_parquet_path(corpus, active_epoch);
            if !parquet_path.exists() {
                continue;
            }

            let table_name =
                SearchPlaneService::local_epoch_engine_table_name(corpus, active_epoch);
            self.service
                .search_engine()
                .ensure_parquet_table_registered(table_name.as_str(), parquet_path.as_path(), &[])
                .await
                .map_err(|error| {
                    format!(
                        "studio SQL Flight provider failed to register `{table_name}` for corpus `{corpus}`: {error}"
                    )
                })?;
            table_names.insert(table_name);
        }

        Ok(())
    }

    async fn register_repo_tables(&self, table_names: &mut BTreeSet<String>) -> Result<(), String> {
        let repo_records = self.service.repo_corpus_snapshot_for_reads().await;
        for ((corpus, _repo_id), record) in repo_records {
            let Some(publication) = record.publication else {
                continue;
            };
            if !publication.is_datafusion_readable() {
                continue;
            }

            let parquet_path = self
                .service
                .repo_publication_parquet_path(corpus, publication.table_name.as_str());
            if !parquet_path.exists() {
                continue;
            }

            let table_name = SearchPlaneService::repo_publication_engine_table_name(
                corpus,
                publication.publication_id.as_str(),
            );
            self.service
                .search_engine()
                .ensure_parquet_table_registered(table_name.as_str(), parquet_path.as_path(), &[])
                .await
                .map_err(|error| {
                    format!(
                        "studio SQL Flight provider failed to register `{table_name}` for corpus `{corpus}`: {error}"
                    )
                })?;
            table_names.insert(table_name);
        }

        Ok(())
    }
}

impl std::fmt::Debug for StudioSqlFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioSqlFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct StudioSqlFlightMetadata {
    registered_tables: Vec<String>,
    registered_table_count: usize,
    result_batch_count: usize,
    result_row_count: usize,
}

#[async_trait]
impl SqlFlightRouteProvider for StudioSqlFlightRouteProvider {
    async fn sql_query_batches(&self, query_text: &str) -> Result<SqlFlightRouteResponse, String> {
        let registered_tables = self.register_readable_tables().await?;
        let engine_batches = self
            .service
            .search_engine()
            .sql_batches(query_text)
            .await
            .map_err(|error| {
                format!("studio SQL Flight provider failed to execute `{query_text}`: {error}")
            })?;
        let result_row_count = engine_batches
            .iter()
            .map(xiuxian_vector::EngineRecordBatch::num_rows)
            .sum();
        let batches = engine_batches_to_lance_batches(engine_batches.as_slice()).map_err(|error| {
            format!(
                "studio SQL Flight provider failed to convert SQL response batches for `{query_text}`: {error}"
            )
        })?;
        let app_metadata = serde_json::to_vec(&StudioSqlFlightMetadata {
            registered_table_count: registered_tables.len(),
            registered_tables,
            result_batch_count: batches.len(),
            result_row_count,
        })
        .map_err(|error| {
            format!("studio SQL Flight provider failed to encode app metadata: {error}")
        })?;

        Ok(SqlFlightRouteResponse::new(batches).with_app_metadata(app_metadata))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::gateway::studio::types::{ReferenceSearchHit, StudioNavigationTarget};
    use crate::search_plane::{
        BeginBuildDecision, SearchMaintenancePolicy, SearchManifestKeyspace,
        reference_occurrence_batches, reference_occurrence_schema,
    };
    use xiuxian_vector::ColumnarScanOptions;
    use xiuxian_wendao_runtime::transport::SqlFlightRouteProvider;

    use super::{
        SearchCorpusKind, SearchPlaneService, StudioSqlFlightMetadata, StudioSqlFlightRouteProvider,
    };

    fn fixture_service(temp_dir: &tempfile::TempDir) -> SearchPlaneService {
        SearchPlaneService::with_paths(
            PathBuf::from("/tmp/project"),
            temp_dir.path().join("search_plane"),
            SearchManifestKeyspace::new("xiuxian:test:studio_sql_flight"),
            SearchMaintenancePolicy::default(),
        )
    }

    fn sample_hit(name: &str, path: &str, line: usize) -> ReferenceSearchHit {
        ReferenceSearchHit {
            name: name.to_string(),
            path: path.to_string(),
            language: "rust".to_string(),
            crate_name: "kernel".to_string(),
            project_name: None,
            root_label: None,
            line,
            column: 5,
            line_text: format!("let _value = {name};"),
            navigation_target: StudioNavigationTarget {
                path: path.to_string(),
                category: "doc".to_string(),
                project_name: None,
                root_label: None,
                line: Some(line),
                line_end: Some(line),
                column: Some(5),
            },
            score: 0.0,
        }
    }

    #[tokio::test]
    async fn studio_sql_flight_provider_queries_registered_reference_occurrence_table() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let service = fixture_service(&temp_dir);
        let lease = match service.coordinator().begin_build(
            SearchCorpusKind::ReferenceOccurrence,
            "fp-sql-1",
            SearchCorpusKind::ReferenceOccurrence.schema_version(),
        ) {
            BeginBuildDecision::Started(lease) => lease,
            other => panic!("unexpected begin decision: {other:?}"),
        };
        let hits = vec![
            sample_hit("AlphaService", "src/lib.rs", 10),
            sample_hit("BetaThing", "src/beta.rs", 20),
        ];
        let store = service
            .open_store(SearchCorpusKind::ReferenceOccurrence)
            .await
            .unwrap_or_else(|error| panic!("open store: {error}"));
        let table_name =
            SearchPlaneService::table_name(SearchCorpusKind::ReferenceOccurrence, lease.epoch);
        store
            .replace_record_batches(
                table_name.as_str(),
                reference_occurrence_schema(),
                reference_occurrence_batches(&hits)
                    .unwrap_or_else(|error| panic!("reference occurrence batches: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("replace record batches: {error}"));
        store
            .write_vector_store_table_to_parquet_file(
                table_name.as_str(),
                service
                    .local_epoch_parquet_path(SearchCorpusKind::ReferenceOccurrence, lease.epoch)
                    .as_path(),
                ColumnarScanOptions::default(),
            )
            .await
            .unwrap_or_else(|error| panic!("export parquet: {error}"));
        service
            .coordinator()
            .publish_ready(&lease, hits.len() as u64, 1);

        let provider = StudioSqlFlightRouteProvider::new(service.clone());
        let engine_table_name = SearchPlaneService::local_epoch_engine_table_name(
            SearchCorpusKind::ReferenceOccurrence,
            lease.epoch,
        );
        let response = provider
            .sql_query_batches(
                format!("SELECT name, path FROM {engine_table_name} WHERE name = 'AlphaService'")
                    .as_str(),
            )
            .await
            .unwrap_or_else(|error| panic!("SQL query batches: {error}"));

        assert_eq!(response.batches.len(), 1);
        assert_eq!(response.batches[0].num_rows(), 1);
        assert!(
            response.batches[0].column_by_name("name").is_some(),
            "name column should exist"
        );
        assert!(
            response.batches[0].column_by_name("path").is_some(),
            "path column should exist"
        );

        let app_metadata: StudioSqlFlightMetadata =
            serde_json::from_slice(response.app_metadata.as_slice())
                .unwrap_or_else(|error| panic!("decode app metadata: {error}"));
        assert_eq!(
            app_metadata,
            StudioSqlFlightMetadata {
                registered_tables: vec![engine_table_name],
                registered_table_count: 1,
                result_batch_count: 1,
                result_row_count: 1,
            }
        );
    }
}
