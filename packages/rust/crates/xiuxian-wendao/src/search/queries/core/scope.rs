use xiuxian_vector::SearchEngineContext;

use crate::search::SearchPlaneService;
use crate::search::queries::sql::registration::{SqlQuerySurface, register_sql_query_surface};

/// Shared request-scoped query core over the visible search-plane data.
pub(crate) struct QueryCore {
    query_engine: SearchEngineContext,
    surface: SqlQuerySurface,
}

impl QueryCore {
    fn new(query_engine: SearchEngineContext, surface: SqlQuerySurface) -> Self {
        Self {
            query_engine,
            surface,
        }
    }

    pub(crate) fn engine(&self) -> &SearchEngineContext {
        &self.query_engine
    }

    pub(crate) fn surface(&self) -> &SqlQuerySurface {
        &self.surface
    }
}

pub(crate) async fn open_query_core(service: &SearchPlaneService) -> Result<QueryCore, String> {
    let (query_engine, surface) = register_sql_query_surface(service).await?;
    Ok(QueryCore::new(query_engine, surface))
}
