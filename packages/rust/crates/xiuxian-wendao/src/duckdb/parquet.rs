use std::path::Path;
#[cfg(feature = "duckdb")]
use std::sync::{Arc, Mutex, MutexGuard};

use xiuxian_vector::{EngineRecordBatch, SearchEngineContext, VectorStoreError};

#[cfg(feature = "duckdb")]
use super::connection::SearchDuckDbConnection;
use super::engine::LocalRelationEngineKind;
#[cfg(feature = "duckdb")]
use super::engine::{
    build_drop_duckdb_registered_relation_sql, ensure_duckdb_identifier, quoted_duckdb_identifier,
};
#[cfg(feature = "duckdb")]
use super::runtime::resolve_search_duckdb_runtime;
#[cfg(feature = "duckdb")]
use xiuxian_wendao_runtime::config::SearchDuckDbRuntimeConfig;

/// DataFusion-backed repo publication Parquet query engine.
#[derive(Clone)]
pub struct DataFusionParquetQueryEngine {
    context: SearchEngineContext,
}

impl DataFusionParquetQueryEngine {
    /// Wrap one existing `DataFusion` search-engine context for Parquet reads.
    #[must_use]
    pub fn new(context: SearchEngineContext) -> Self {
        Self { context }
    }
}

/// DuckDB-backed repo publication Parquet query engine.
#[cfg(feature = "duckdb")]
pub struct DuckDbParquetQueryEngine {
    connection: Mutex<SearchDuckDbConnection>,
}

#[cfg(feature = "duckdb")]
impl DuckDbParquetQueryEngine {
    /// Open a `DuckDB`-backed Parquet query engine from one resolved runtime.
    ///
    /// # Errors
    ///
    /// Returns an error when the configured `DuckDB` connection cannot be
    /// initialized.
    pub fn from_runtime(runtime: SearchDuckDbRuntimeConfig) -> Result<Self, VectorStoreError> {
        let connection =
            SearchDuckDbConnection::from_runtime(runtime).map_err(VectorStoreError::General)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn lock_connection(&self) -> Result<MutexGuard<'_, SearchDuckDbConnection>, VectorStoreError> {
        self.connection.lock().map_err(|_| {
            VectorStoreError::General("search DuckDB connection mutex is poisoned".to_string())
        })
    }
}

/// Narrow Parquet-backed query-engine seam for repo-backed gateway search.
#[derive(Clone)]
pub enum ParquetQueryEngine {
    /// Execute repo-backed Parquet reads through the existing `DataFusion` lane.
    DataFusion(DataFusionParquetQueryEngine),
    #[cfg(feature = "duckdb")]
    /// Execute repo-backed Parquet reads through the local `DuckDB` lane.
    DuckDb(Arc<DuckDbParquetQueryEngine>),
}

impl ParquetQueryEngine {
    /// Build one configured Parquet query engine for repo-backed reads.
    ///
    /// When `search.duckdb.enabled` is true and the `duckdb` feature is
    /// compiled in, repo-backed reads become `DuckDB`-backed. Otherwise the
    /// current `DataFusion` search engine remains the fallback.
    ///
    /// # Errors
    ///
    /// Returns an error when runtime-selected `DuckDB` initialization fails.
    #[cfg(feature = "duckdb")]
    pub fn configured(default_context: SearchEngineContext) -> Result<Self, VectorStoreError> {
        #[cfg(feature = "duckdb")]
        {
            let runtime = resolve_search_duckdb_runtime();
            if runtime.enabled {
                return DuckDbParquetQueryEngine::from_runtime(runtime)
                    .map(|engine| Self::DuckDb(Arc::new(engine)));
            }
        }

        Ok(Self::DataFusion(DataFusionParquetQueryEngine::new(
            default_context,
        )))
    }

    /// Build one configured Parquet query engine for repo-backed reads.
    ///
    /// Without the `duckdb` feature compiled in, the query engine always uses
    /// the current `DataFusion` backend.
    #[cfg(not(feature = "duckdb"))]
    #[must_use]
    pub fn configured(default_context: SearchEngineContext) -> Self {
        Self::DataFusion(DataFusionParquetQueryEngine::new(default_context))
    }

    /// Build one explicit `DuckDB`-backed Parquet query engine from one runtime.
    ///
    /// # Errors
    ///
    /// Returns an error when the provided `DuckDB` runtime cannot be
    /// initialized.
    #[cfg(feature = "duckdb")]
    pub fn duckdb_from_runtime(
        runtime: SearchDuckDbRuntimeConfig,
    ) -> Result<Self, VectorStoreError> {
        DuckDbParquetQueryEngine::from_runtime(runtime).map(|engine| Self::DuckDb(Arc::new(engine)))
    }

    /// Report the active engine kind.
    #[must_use]
    pub fn kind(&self) -> LocalRelationEngineKind {
        match self {
            Self::DataFusion(_) => LocalRelationEngineKind::DataFusion,
            #[cfg(feature = "duckdb")]
            Self::DuckDb(_) => LocalRelationEngineKind::DuckDb,
        }
    }

    /// Ensure one published Parquet table is queryable through this engine.
    ///
    /// # Errors
    ///
    /// Returns an error when table registration fails.
    pub async fn ensure_parquet_table_registered(
        &self,
        table_name: &str,
        table_path: &Path,
    ) -> Result<(), VectorStoreError> {
        match self {
            Self::DataFusion(engine) => {
                engine
                    .context
                    .ensure_parquet_table_registered(table_name, table_path, &[])
                    .await
            }
            #[cfg(feature = "duckdb")]
            Self::DuckDb(engine) => engine.register_parquet_view(table_name, table_path),
        }
    }

    /// Execute one SQL query and collect Arrow batches.
    ///
    /// # Errors
    ///
    /// Returns an error when planning or execution fails.
    pub async fn query_batches(
        &self,
        sql: &str,
    ) -> Result<Vec<EngineRecordBatch>, VectorStoreError> {
        match self {
            Self::DataFusion(engine) => engine.context.sql_batches(sql).await,
            #[cfg(feature = "duckdb")]
            Self::DuckDb(engine) => engine.query_batches(sql),
        }
    }
}

#[cfg(feature = "duckdb")]
impl DuckDbParquetQueryEngine {
    fn register_parquet_view(
        &self,
        table_name: &str,
        table_path: &Path,
    ) -> Result<(), VectorStoreError> {
        let sql = build_duckdb_parquet_view_sql(table_name, table_path)?;
        let guard = self.lock_connection()?;
        guard.connection().execute_batch(sql.as_str()).map_err(|error| {
            VectorStoreError::General(format!(
                "failed to register DuckDB repo publication parquet view `{table_name}`: {error}"
            ))
        })?;
        Ok(())
    }

    fn query_batches(&self, sql: &str) -> Result<Vec<EngineRecordBatch>, VectorStoreError> {
        let guard = self.lock_connection()?;
        let mut statement = guard.connection().prepare(sql).map_err(|error| {
            VectorStoreError::General(format!(
                "failed to prepare DuckDB repo publication SQL `{sql}`: {error}"
            ))
        })?;
        let batches = statement
            .query_arrow([])
            .map_err(|error| {
                VectorStoreError::General(format!(
                    "DuckDB repo publication SQL execution failed for `{sql}`: {error}"
                ))
            })?
            .collect::<Vec<_>>();
        Ok(batches)
    }
}

#[cfg(feature = "duckdb")]
fn build_duckdb_parquet_view_sql(
    table_name: &str,
    table_path: &Path,
) -> Result<String, VectorStoreError> {
    ensure_duckdb_identifier(table_name, "table").map_err(VectorStoreError::General)?;
    let quoted_table_name = quoted_duckdb_identifier(table_name);
    let escaped_path = table_path.to_string_lossy().replace('\'', "''");
    Ok(format!(
        "{drop_sql}\nCREATE TEMP VIEW {quoted_table_name} AS SELECT * FROM read_parquet('{escaped_path}');",
        drop_sql = build_drop_duckdb_registered_relation_sql(table_name),
    ))
}
