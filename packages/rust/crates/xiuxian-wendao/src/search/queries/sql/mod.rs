/// Shared bounded-work markdown SQL surface for workdir-local retrieval.
pub mod bounded_work_markdown;
pub(crate) mod execution;
pub(crate) mod provider;
pub(crate) mod registration;

pub use self::execution::{
    SqlBatchPayload, SqlColumnPayload, SqlQueryMetadata, SqlQueryPayload, query_sql_payload,
};
pub(crate) use self::execution::{engine_batches_rows_payload, execute_sql_query};
pub(crate) use self::registration::SqlQuerySurface;

#[cfg(test)]
#[path = "../../../../tests/unit/search/queries/sql/mod.rs"]
mod tests;
