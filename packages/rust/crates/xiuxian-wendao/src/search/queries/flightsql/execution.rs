use std::path::PathBuf;
use std::sync::Arc;

use arrow::array::ArrayRef;
use arrow::compute::cast;
use arrow::datatypes::{DataType, Field, Schema};
use xiuxian_vector::EngineRecordBatch;

use crate::duckdb::{LocalRelationEngineKind, ParquetQueryEngine};
use crate::search::queries::SearchQueryService;
use crate::search::queries::sql::execute_sql_query;
use crate::search::{SearchCorpusKind, SearchPlaneService};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum FlightSqlStatementRoute {
    SharedSql,
    LocalParquet {
        corpus: SearchCorpusKind,
        table_name: String,
        engine_kind: LocalRelationEngineKind,
    },
}

pub(super) struct FlightSqlStatementExecution {
    pub(super) route: FlightSqlStatementRoute,
    pub(super) batches: Vec<EngineRecordBatch>,
}

pub(super) async fn execute_flightsql_statement_query(
    service: &SearchQueryService,
    query_engine: Option<&ParquetQueryEngine>,
    query_text: &str,
) -> Result<FlightSqlStatementExecution, String> {
    if let Some(result) = try_execute_published_parquet_statement_query(
        service.search_plane(),
        query_engine,
        query_text,
    )
    .await?
    {
        return Ok(result);
    }

    let (_metadata, batches) = execute_sql_query(service, query_text).await?.into_parts();
    Ok(FlightSqlStatementExecution {
        route: FlightSqlStatementRoute::SharedSql,
        batches,
    })
}

async fn try_execute_published_parquet_statement_query(
    service: &SearchPlaneService,
    query_engine: Option<&ParquetQueryEngine>,
    query_text: &str,
) -> Result<Option<FlightSqlStatementExecution>, String> {
    let Some(target) = resolve_published_parquet_statement_target(service, query_text).await else {
        return Ok(None);
    };
    let query_engine = query_engine
        .cloned()
        .map(Ok)
        .unwrap_or_else(|| configured_parquet_query_engine(service))?;
    let engine_kind = query_engine.kind();
    query_engine
        .ensure_parquet_table_registered(target.table_name.as_str(), target.parquet_path.as_path())
        .await
        .map_err(|error| {
            format!(
                "FlightSQL failed to register published parquet table `{}`: {error}",
                target.table_name
            )
        })?;
    let batches = normalize_flightsql_statement_batches(
        query_engine
            .query_batches(query_text)
            .await
            .map_err(|error| {
                format!(
                    "FlightSQL published parquet statement execution failed for `{query_text}`: {error}"
                )
            })?,
    )?;
    Ok(Some(FlightSqlStatementExecution {
        route: FlightSqlStatementRoute::LocalParquet {
            corpus: target.corpus,
            table_name: target.table_name,
            engine_kind,
        },
        batches,
    }))
}

#[cfg(feature = "duckdb")]
pub(super) fn configured_parquet_query_engine(
    service: &SearchPlaneService,
) -> Result<ParquetQueryEngine, String> {
    ParquetQueryEngine::configured(service.search_engine().clone()).map_err(|error| {
        format!("FlightSQL failed to configure published parquet query engine: {error}")
    })
}

#[cfg(not(feature = "duckdb"))]
pub(super) fn configured_parquet_query_engine(
    service: &SearchPlaneService,
) -> Result<ParquetQueryEngine, String> {
    Ok(ParquetQueryEngine::configured(
        service.search_engine().clone(),
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedLocalParquetStatementTarget {
    corpus: SearchCorpusKind,
    table_name: String,
    parquet_path: PathBuf,
}

async fn resolve_published_parquet_statement_target(
    service: &SearchPlaneService,
    query_text: &str,
) -> Option<ResolvedLocalParquetStatementTarget> {
    let table_name = extract_single_source_table_name(query_text)?;
    if let Some(target) = resolve_active_local_corpus(service, table_name.as_str()) {
        return Some(target);
    }
    if let Some(target) =
        resolve_local_symbol_source_table_statement_target(service, table_name.as_str())
    {
        return Some(target);
    }
    resolve_repo_source_table_statement_target(service, table_name.as_str()).await
}

fn resolve_active_local_corpus(
    service: &SearchPlaneService,
    table_name: &str,
) -> Option<ResolvedLocalParquetStatementTarget> {
    let corpus = match table_name {
        "reference_occurrence" => SearchCorpusKind::ReferenceOccurrence,
        "attachment" => SearchCorpusKind::Attachment,
        "knowledge_section" => SearchCorpusKind::KnowledgeSection,
        _ => return None,
    };
    let active_epoch = service.coordinator().status_for(corpus).active_epoch?;
    let parquet_path = service.local_epoch_parquet_path(corpus, active_epoch);
    if !parquet_path.exists() {
        return None;
    }
    Some(ResolvedLocalParquetStatementTarget {
        corpus,
        table_name: table_name.to_string(),
        parquet_path,
    })
}

fn resolve_local_symbol_source_table_statement_target(
    service: &SearchPlaneService,
    table_name: &str,
) -> Option<ResolvedLocalParquetStatementTarget> {
    let active_epoch = service
        .coordinator()
        .status_for(SearchCorpusKind::LocalSymbol)
        .active_epoch?;
    let source_table_names =
        service.local_epoch_table_names_for_reads(SearchCorpusKind::LocalSymbol, active_epoch);
    if !source_table_names.iter().any(|source| source == table_name) {
        return None;
    }
    let parquet_path = service.local_table_parquet_path(SearchCorpusKind::LocalSymbol, table_name);
    if !parquet_path.exists() {
        return None;
    }
    Some(ResolvedLocalParquetStatementTarget {
        corpus: SearchCorpusKind::LocalSymbol,
        table_name: table_name.to_string(),
        parquet_path,
    })
}

async fn resolve_repo_source_table_statement_target(
    service: &SearchPlaneService,
    table_name: &str,
) -> Option<ResolvedLocalParquetStatementTarget> {
    if !table_name.starts_with("repo_entity_repo_")
        && !table_name.starts_with("repo_content_chunk_repo_")
    {
        return None;
    }

    let repo_records = service.repo_corpus_snapshot_for_reads().await;
    for ((corpus, repo_id), record) in repo_records {
        let expected_table_name = match corpus {
            SearchCorpusKind::RepoEntity => {
                SearchPlaneService::repo_entity_table_name(repo_id.as_str())
            }
            SearchCorpusKind::RepoContentChunk => {
                SearchPlaneService::repo_content_chunk_table_name(repo_id.as_str())
            }
            SearchCorpusKind::LocalSymbol
            | SearchCorpusKind::KnowledgeSection
            | SearchCorpusKind::Attachment
            | SearchCorpusKind::ReferenceOccurrence => continue,
        };
        if expected_table_name != table_name {
            continue;
        }

        let publication = record.publication?;
        if !publication.is_datafusion_readable() {
            return None;
        }
        let parquet_path =
            service.repo_publication_parquet_path(corpus, publication.table_name.as_str());
        if !parquet_path.exists() {
            return None;
        }

        return Some(ResolvedLocalParquetStatementTarget {
            corpus,
            table_name: table_name.to_string(),
            parquet_path,
        });
    }

    None
}

fn extract_single_source_table_name(query_text: &str) -> Option<String> {
    let trimmed = query_text.trim();
    if trimmed.is_empty() || (trimmed.contains(';') && !trimmed.ends_with(';')) {
        return None;
    }

    let tokens = trimmed
        .trim_end_matches(';')
        .split_whitespace()
        .collect::<Vec<_>>();
    let first = tokens.first()?;
    if !first.eq_ignore_ascii_case("select") {
        return None;
    }

    for token in &tokens {
        if token.eq_ignore_ascii_case("join")
            || token.eq_ignore_ascii_case("union")
            || token.eq_ignore_ascii_case("intersect")
            || token.eq_ignore_ascii_case("except")
        {
            return None;
        }
    }

    let from_index = tokens
        .iter()
        .position(|token| token.eq_ignore_ascii_case("from"))?;
    let table_token = tokens.get(from_index + 1)?.trim_end_matches(',');
    if table_token.is_empty()
        || table_token.starts_with('(')
        || table_token.contains(',')
        || table_token.contains('.')
    {
        return None;
    }
    normalize_sql_identifier(table_token)
}

fn normalize_sql_identifier(token: &str) -> Option<String> {
    if token.len() >= 2 && token.starts_with('"') && token.ends_with('"') {
        let inner = token[1..token.len() - 1].replace("\"\"", "\"");
        if inner.is_empty() || inner.contains('.') {
            return None;
        }
        return Some(inner);
    }

    if token
        .chars()
        .all(|char| char == '_' || char.is_ascii_alphanumeric())
    {
        return Some(token.to_ascii_lowercase());
    }

    None
}

fn normalize_flightsql_statement_batches(
    batches: Vec<EngineRecordBatch>,
) -> Result<Vec<EngineRecordBatch>, String> {
    batches
        .into_iter()
        .map(normalize_flightsql_statement_batch)
        .collect()
}

fn normalize_flightsql_statement_batch(
    batch: EngineRecordBatch,
) -> Result<EngineRecordBatch, String> {
    let schema = batch.schema();
    let mut changed = false;
    let mut fields = Vec::with_capacity(schema.fields().len());
    let mut columns = Vec::with_capacity(batch.num_columns());

    for (field, column) in schema.fields().iter().zip(batch.columns().iter()) {
        let (normalized_field, normalized_column, column_changed) =
            normalize_flightsql_statement_column(field.as_ref(), column.clone())?;
        fields.push(normalized_field);
        columns.push(normalized_column);
        changed |= column_changed;
    }

    if !changed {
        return Ok(batch);
    }

    EngineRecordBatch::try_new(
        Arc::new(Schema::new_with_metadata(fields, schema.metadata().clone())),
        columns,
    )
    .map_err(|error| format!("FlightSQL failed to rebuild normalized statement batch: {error}"))
}

fn normalize_flightsql_statement_column(
    field: &Field,
    column: ArrayRef,
) -> Result<(Field, ArrayRef, bool), String> {
    match field.data_type() {
        DataType::Utf8 | DataType::LargeUtf8 => {
            let normalized_column =
                cast(column.as_ref(), &DataType::Utf8View).map_err(|error| {
                    format!(
                        "FlightSQL failed to normalize string column `{}` to Utf8View: {error}",
                        field.name()
                    )
                })?;
            Ok((
                Field::new(field.name(), DataType::Utf8View, field.is_nullable())
                    .with_metadata(field.metadata().clone()),
                normalized_column,
                true,
            ))
        }
        _ => Ok((field.clone(), column, false)),
    }
}
