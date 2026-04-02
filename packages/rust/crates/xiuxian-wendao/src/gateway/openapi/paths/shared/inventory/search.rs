use crate::gateway::openapi::paths::*;

pub(super) const SEARCH: RouteContract = RouteContract {
    axum_path: API_SEARCH_AXUM_PATH,
    openapi_path: API_SEARCH_OPENAPI_PATH,
    methods: &["get"],
    path_params: &[],
};

pub(super) const SEARCH_DEFINITION: RouteContract = RouteContract {
    axum_path: API_SEARCH_DEFINITION_AXUM_PATH,
    openapi_path: API_SEARCH_DEFINITION_OPENAPI_PATH,
    methods: &["get"],
    path_params: &[],
};

pub(super) const SEARCH_AUTOCOMPLETE: RouteContract = RouteContract {
    axum_path: API_SEARCH_AUTOCOMPLETE_AXUM_PATH,
    openapi_path: API_SEARCH_AUTOCOMPLETE_OPENAPI_PATH,
    methods: &["get"],
    path_params: &[],
};

pub(super) const SEARCH_INDEX_STATUS: RouteContract = RouteContract {
    axum_path: API_SEARCH_INDEX_STATUS_AXUM_PATH,
    openapi_path: API_SEARCH_INDEX_STATUS_OPENAPI_PATH,
    methods: &["get"],
    path_params: &[],
};
