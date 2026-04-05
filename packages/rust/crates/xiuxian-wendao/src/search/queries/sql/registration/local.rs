use std::collections::BTreeMap;

use crate::search_plane::{SearchCorpusKind, SearchPlaneService};
use xiuxian_vector::SearchEngineContext;

use super::{RegisteredSqlTable, naming};

pub(super) async fn register_local_tables(
    service: &SearchPlaneService,
    query_engine: &SearchEngineContext,
    tables: &mut BTreeMap<String, RegisteredSqlTable>,
) -> Result<(), String> {
    for corpus in SearchCorpusKind::ALL
        .into_iter()
        .filter(|corpus| !corpus.is_repo_backed())
    {
        let status = service.coordinator().status_for(corpus);
        let Some(active_epoch) = status.active_epoch else {
            continue;
        };

        if corpus == SearchCorpusKind::LocalSymbol {
            for table_name in service.local_epoch_table_names_for_reads(corpus, active_epoch) {
                let parquet_path = service.local_table_parquet_path(corpus, table_name.as_str());
                if !parquet_path.exists() {
                    continue;
                }
                query_engine
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
                tables.insert(
                    table_name.clone(),
                    RegisteredSqlTable::local(corpus, table_name.clone(), table_name),
                );
            }
            continue;
        }

        let parquet_path = service.local_epoch_parquet_path(corpus, active_epoch);
        if !parquet_path.exists() {
            continue;
        }

        let engine_table_name =
            SearchPlaneService::local_epoch_engine_table_name(corpus, active_epoch);
        let sql_table_name = naming::local_sql_table_name(corpus, engine_table_name.as_str());
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
            RegisteredSqlTable::local(corpus, sql_table_name, engine_table_name),
        );
    }

    Ok(())
}
