use crate::gateway::openapi::paths::*;

pub(super) const ANALYSIS_MARKDOWN: RouteContract = RouteContract {
    axum_path: API_ANALYSIS_MARKDOWN_AXUM_PATH,
    openapi_path: API_ANALYSIS_MARKDOWN_OPENAPI_PATH,
    methods: &["get"],
    path_params: &[],
};

pub(super) const ANALYSIS_CODE_AST: RouteContract = RouteContract {
    axum_path: API_ANALYSIS_CODE_AST_AXUM_PATH,
    openapi_path: API_ANALYSIS_CODE_AST_OPENAPI_PATH,
    methods: &["get"],
    path_params: &[],
};
