use std::sync::Arc;

use arrow::array::{Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use datafusion::datasource::MemTable;
use xiuxian_vector::SearchEngineContext;

use super::rows::BoundedWorkMarkdownRow;

pub(crate) fn bounded_work_markdown_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new("path", DataType::Utf8, false),
        Field::new("surface", DataType::Utf8, false),
        Field::new("heading_path", DataType::Utf8, false),
        Field::new("title", DataType::Utf8, false),
        Field::new("level", DataType::Int64, false),
        Field::new("skeleton", DataType::Utf8, false),
        Field::new("body", DataType::Utf8, false),
    ]))
}

pub(crate) fn register_markdown_mem_table(
    query_engine: &SearchEngineContext,
    table_name: &str,
    rows: &[BoundedWorkMarkdownRow],
) -> Result<(), String> {
    let schema = bounded_work_markdown_schema();
    let batch = RecordBatch::try_new(
        Arc::clone(&schema),
        vec![
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| Some(row.path.as_str()))
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| Some(row.surface.as_str()))
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| Some(row.heading_path.as_str()))
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| Some(row.title.as_str()))
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Int64Array::from(
                rows.iter().map(|row| Some(row.level)).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| Some(row.skeleton.as_str()))
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| Some(row.body.as_str()))
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| format!("failed to build bounded work markdown batch: {error}"))?;
    let mem_table = MemTable::try_new(schema, vec![vec![batch]])
        .map_err(|error| format!("failed to build bounded work markdown mem table: {error}"))?;
    let _ = query_engine.session().deregister_table(table_name);
    query_engine
        .session()
        .register_table(table_name, Arc::new(mem_table))
        .map_err(|error| {
            format!("failed to register bounded work markdown table `{table_name}`: {error}")
        })?;
    Ok(())
}
