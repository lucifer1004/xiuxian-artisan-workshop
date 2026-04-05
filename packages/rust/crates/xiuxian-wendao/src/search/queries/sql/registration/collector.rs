use std::collections::BTreeMap;

use crate::search_plane::SearchPlaneService;
use xiuxian_vector::SearchEngineContext;

use super::catalog::{
    collect_registered_columns, register_columns_catalog_table, register_tables_catalog_table,
    register_view_sources_catalog_table,
};
use super::{
    RegisteredSqlTable, STUDIO_SQL_CATALOG_TABLE_NAME, STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME,
    STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME, SqlQuerySurface, local, repo, session, views,
};

pub(crate) async fn register_sql_query_surface(
    service: &SearchPlaneService,
) -> Result<(SearchEngineContext, SqlQuerySurface), String> {
    let query_engine = session::new_sql_query_engine();
    let mut tables = BTreeMap::<String, RegisteredSqlTable>::new();
    local::register_local_tables(service, &query_engine, &mut tables).await?;
    let mut view_sources = views::register_local_logical_views(&query_engine, &mut tables).await?;
    repo::register_repo_tables(service, &query_engine, &mut tables).await?;
    view_sources.extend(views::register_repo_logical_views(&query_engine, &mut tables).await?);
    tables.insert(
        STUDIO_SQL_CATALOG_TABLE_NAME.to_string(),
        RegisteredSqlTable::system(STUDIO_SQL_CATALOG_TABLE_NAME),
    );
    tables.insert(
        STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME.to_string(),
        RegisteredSqlTable::system(STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME),
    );
    tables.insert(
        STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME.to_string(),
        RegisteredSqlTable::system(STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME),
    );
    let tables = tables.into_values().collect::<Vec<_>>();
    register_tables_catalog_table(&query_engine, tables.as_slice())?;
    register_view_sources_catalog_table(&query_engine, view_sources.as_slice())?;
    let columns = collect_registered_columns(&query_engine, tables.as_slice()).await?;
    register_columns_catalog_table(&query_engine, columns.as_slice())?;
    let surface = SqlQuerySurface::new(
        STUDIO_SQL_CATALOG_TABLE_NAME,
        STUDIO_SQL_COLUMNS_CATALOG_TABLE_NAME,
        STUDIO_SQL_VIEW_SOURCES_CATALOG_TABLE_NAME,
        tables,
        columns,
        view_sources,
    );
    Ok((query_engine, surface))
}
