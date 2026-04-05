use std::path::PathBuf;

use crate::search::queries::core::scope::{QueryCore, open_query_core};
use crate::search_plane::SearchPlaneService;

/// Canonical shared-query service owner above the request-scoped query core.
#[derive(Clone)]
pub struct SearchQueryService {
    search_plane: SearchPlaneService,
}

impl SearchQueryService {
    /// Create a shared query service over the provided search plane.
    #[must_use]
    pub fn new(search_plane: SearchPlaneService) -> Self {
        Self { search_plane }
    }

    /// Create a shared query service from one project root.
    #[must_use]
    pub fn from_project_root(project_root: impl Into<PathBuf>) -> Self {
        Self::new(SearchPlaneService::new(project_root.into()))
    }

    pub(crate) fn search_plane(&self) -> &SearchPlaneService {
        &self.search_plane
    }

    pub(crate) async fn open_core(&self) -> Result<QueryCore, String> {
        open_query_core(self.search_plane()).await
    }
}

impl From<SearchPlaneService> for SearchQueryService {
    fn from(search_plane: SearchPlaneService) -> Self {
        Self::new(search_plane)
    }
}
