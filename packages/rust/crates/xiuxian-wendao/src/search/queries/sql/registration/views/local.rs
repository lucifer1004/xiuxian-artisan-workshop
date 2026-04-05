use std::collections::BTreeMap;

use xiuxian_vector::SearchEngineContext;

use crate::search_plane::SearchCorpusKind;

use super::super::{RegisteredSqlTable, RegisteredSqlViewSource, naming};

pub(crate) async fn register_local_logical_views(
    query_engine: &SearchEngineContext,
    tables: &mut BTreeMap<String, RegisteredSqlTable>,
) -> Result<Vec<RegisteredSqlViewSource>, String> {
    let corpus = SearchCorpusKind::LocalSymbol;
    let mut source_tables = tables
        .values()
        .filter(|table| table.scope == "local" && table.corpus == corpus.to_string())
        .cloned()
        .collect::<Vec<_>>();
    source_tables.sort_by(|left, right| left.sql_table_name.cmp(&right.sql_table_name));
    if source_tables.is_empty() {
        return Ok(Vec::new());
    }

    let logical_view_name = naming::local_logical_view_name(corpus);
    let view_sql = build_local_logical_view_sql(logical_view_name.as_str(), &source_tables);
    query_engine
        .session()
        .sql(view_sql.as_str())
        .await
        .map_err(|error| {
            format!(
                "studio SQL Flight provider failed to register local logical view `{logical_view_name}`: {error}"
            )
        })?;

    let view_sources = source_tables
        .iter()
        .enumerate()
        .map(|(index, table)| {
            RegisteredSqlViewSource::logical(logical_view_name.as_str(), table, index + 1)
        })
        .collect::<Vec<_>>();
    tables.insert(
        logical_view_name.clone(),
        RegisteredSqlTable::local_logical(corpus, logical_view_name, source_tables.len()),
    );

    Ok(view_sources)
}

fn build_local_logical_view_sql(view_name: &str, source_tables: &[RegisteredSqlTable]) -> String {
    let union_query = source_tables
        .iter()
        .enumerate()
        .map(|(index, table)| {
            let source_alias = format!("source_{index}");
            format!(
                "SELECT {source_alias}.* FROM {} AS {source_alias}",
                table.sql_table_name
            )
        })
        .collect::<Vec<_>>()
        .join(" UNION ALL ");
    format!("CREATE VIEW {view_name} AS {union_query}")
}
