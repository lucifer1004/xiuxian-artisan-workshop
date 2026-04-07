use std::collections::BTreeMap;

use crate::search::SearchPlaneService;
use xiuxian_vector::SearchEngineContext;

use super::{RegisteredSqlTable, naming};

pub(super) async fn register_repo_tables(
    service: &SearchPlaneService,
    query_engine: &SearchEngineContext,
    tables: &mut BTreeMap<String, RegisteredSqlTable>,
) -> Result<(), String> {
    let repo_records = service.repo_corpus_snapshot_for_reads().await;
    for ((corpus, repo_id), record) in repo_records {
        let Some(publication) = record.publication else {
            continue;
        };
        if !publication.is_datafusion_readable() {
            continue;
        }

        let parquet_path =
            service.repo_publication_parquet_path(corpus, publication.table_name.as_str());
        if !parquet_path.exists() {
            continue;
        }

        let engine_table_name = SearchPlaneService::repo_publication_engine_table_name(
            corpus,
            publication.publication_id.as_str(),
        );
        let sql_table_name = naming::repo_sql_table_name(corpus, repo_id.as_str());
        query_engine
            .ensure_parquet_table_registered(sql_table_name.as_str(), parquet_path.as_path(), &[])
            .await
            .map_err(|error| {
                format!(
                    "studio SQL Flight provider failed to register `{sql_table_name}` for corpus `{corpus}`: {error}"
                )
            })?;
        tables.insert(
            sql_table_name.clone(),
            RegisteredSqlTable::repo(corpus, repo_id.as_str(), sql_table_name, engine_table_name),
        );
    }

    Ok(())
}
