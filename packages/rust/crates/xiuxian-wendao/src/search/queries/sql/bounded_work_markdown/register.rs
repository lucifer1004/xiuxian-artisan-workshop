use std::path::Path;

use xiuxian_vector::SearchEngineContext;

use super::super::registration::new_sql_query_engine;
use super::discovery::discover_bounded_work_markdown_files;
use super::rows::{BoundedWorkMarkdownRow, build_rows_for_file};
use super::schema::register_markdown_mem_table;

/// The default `DataFusion` table name for bounded-work markdown retrieval.
pub const BOUNDED_WORK_MARKDOWN_TABLE_NAME: &str = "markdown";

/// Build bounded-work markdown rows from the `blueprint/` and `plan/` surfaces.
///
/// # Errors
///
/// Returns an error when a required markdown file cannot be discovered, read,
/// parsed, or normalized into the bounded-work row surface.
pub fn build_bounded_work_markdown_rows(
    root: &Path,
) -> Result<Vec<BoundedWorkMarkdownRow>, String> {
    let files = discover_bounded_work_markdown_files(root)?;
    let mut rows = Vec::new();
    for file in files {
        rows.extend(build_rows_for_file(root, &file)?);
    }
    Ok(rows)
}

/// Register the bounded-work markdown rows into a `DataFusion` session.
///
/// # Errors
///
/// Returns an error when bounded-work markdown rows cannot be built or when
/// the in-memory SQL table cannot be registered into the provided engine.
pub fn register_bounded_work_markdown_table(
    query_engine: &SearchEngineContext,
    root: &Path,
) -> Result<Vec<BoundedWorkMarkdownRow>, String> {
    let rows = build_bounded_work_markdown_rows(root)?;
    register_markdown_mem_table(query_engine, BOUNDED_WORK_MARKDOWN_TABLE_NAME, &rows)?;
    Ok(rows)
}

/// Bootstrap a fresh SQL query engine with the bounded-work `markdown` table.
///
/// # Errors
///
/// Returns an error when the bounded-work markdown rows cannot be built or
/// when the in-memory `markdown` table cannot be registered into the fresh
/// query engine.
pub fn bootstrap_bounded_work_markdown_query_engine(
    root: &Path,
) -> Result<(SearchEngineContext, Vec<BoundedWorkMarkdownRow>), String> {
    let query_engine = new_sql_query_engine();
    let rows = register_bounded_work_markdown_table(&query_engine, root)?;
    Ok((query_engine, rows))
}
