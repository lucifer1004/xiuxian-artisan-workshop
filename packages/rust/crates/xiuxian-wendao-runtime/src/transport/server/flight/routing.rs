use std::sync::Arc;

use tonic::Status;

use crate::transport::query_contract::{
    ANALYSIS_CODE_AST_ROUTE, ANALYSIS_MARKDOWN_ROUTE, GRAPH_NEIGHBORS_ROUTE, QUERY_SQL_ROUTE,
    REPO_SEARCH_ROUTE, SEARCH_AST_ROUTE, SEARCH_ATTACHMENTS_ROUTE, SEARCH_AUTOCOMPLETE_ROUTE,
    SEARCH_DEFINITION_ROUTE, VFS_RESOLVE_ROUTE,
};

use super::super::request_metadata::{
    is_search_family_route, join_sorted_set, validate_attachment_search_request_metadata,
    validate_autocomplete_request_metadata, validate_code_ast_analysis_request_metadata,
    validate_definition_request_metadata, validate_graph_neighbors_request_metadata,
    validate_markdown_analysis_request_metadata, validate_repo_search_request_metadata,
    validate_search_request_metadata, validate_sql_request_metadata,
    validate_vfs_resolve_request_metadata,
};
use super::core::WendaoFlightService;
use super::payload::FlightRoutePayload;

impl WendaoFlightService {
    pub(super) fn route_request_cache_key(
        route: &str,
        metadata: &tonic::metadata::MetadataMap,
    ) -> Result<String, Status> {
        if route == REPO_SEARCH_ROUTE {
            let request = validate_repo_search_request_metadata(metadata)?;
            Ok(format!(
                "{route}|{query_text:?}|{limit}|{}|{}|{}|{}|{}",
                join_sorted_set(&request.language_filters),
                join_sorted_set(&request.path_prefixes),
                join_sorted_set(&request.title_filters),
                join_sorted_set(&request.tag_filters),
                join_sorted_set(&request.filename_filters),
                query_text = request.query_text,
                limit = request.limit,
            ))
        } else if route == SEARCH_ATTACHMENTS_ROUTE {
            let (query_text, limit, ext_filters, kind_filters, case_sensitive) =
                validate_attachment_search_request_metadata(metadata)?;
            Ok(format!(
                "{route}|{query_text:?}|{limit}|{}|{}|{case_sensitive}",
                join_sorted_set(&ext_filters),
                join_sorted_set(&kind_filters),
            ))
        } else if route == SEARCH_AST_ROUTE {
            let (query_text, limit, intent, repo_hint) =
                validate_search_request_metadata(metadata)?;
            Ok(format!(
                "{route}|{query_text:?}|{limit}|{intent:?}|{repo_hint:?}"
            ))
        } else if route == SEARCH_DEFINITION_ROUTE {
            let (query_text, source_path, source_line) =
                validate_definition_request_metadata(metadata)?;
            Ok(format!(
                "{route}|{query_text:?}|{source_path:?}|{source_line:?}"
            ))
        } else if route == SEARCH_AUTOCOMPLETE_ROUTE {
            let (prefix, limit) = validate_autocomplete_request_metadata(metadata)?;
            Ok(format!("{route}|{prefix:?}|{limit}"))
        } else if route == QUERY_SQL_ROUTE {
            let query_text = validate_sql_request_metadata(metadata)?;
            Ok(format!("{route}|{query_text:?}"))
        } else if route == VFS_RESOLVE_ROUTE {
            let path = validate_vfs_resolve_request_metadata(metadata)?;
            Ok(format!("{route}|{path:?}"))
        } else if route == GRAPH_NEIGHBORS_ROUTE {
            let (node_id, direction, hops, limit) =
                validate_graph_neighbors_request_metadata(metadata)?;
            Ok(format!("{route}|{node_id:?}|{direction:?}|{hops}|{limit}"))
        } else if route == ANALYSIS_MARKDOWN_ROUTE {
            let path = validate_markdown_analysis_request_metadata(metadata)?;
            Ok(format!("{route}|{path:?}"))
        } else if route == ANALYSIS_CODE_AST_ROUTE {
            let (path, repo_id, line_hint) = validate_code_ast_analysis_request_metadata(metadata)?;
            Ok(format!("{route}|{path:?}|{repo_id:?}|{line_hint:?}"))
        } else if is_search_family_route(route) {
            let (query_text, limit, intent, repo_hint) =
                validate_search_request_metadata(metadata)?;
            Ok(format!(
                "{route}|{query_text:?}|{limit}|{intent:?}|{repo_hint:?}"
            ))
        } else {
            Err(Status::invalid_argument(format!(
                "unexpected routed Flight request: {route}"
            )))
        }
    }

    pub(super) async fn read_route_payload(
        &self,
        route: &str,
        metadata: &tonic::metadata::MetadataMap,
    ) -> Result<FlightRoutePayload, Status> {
        if route == REPO_SEARCH_ROUTE {
            self.read_repo_search_payload(metadata).await
        } else if route == SEARCH_ATTACHMENTS_ROUTE {
            self.read_attachment_search_payload(route, metadata).await
        } else if route == SEARCH_AST_ROUTE {
            self.read_ast_search_payload(route, metadata).await
        } else if route == SEARCH_DEFINITION_ROUTE {
            self.read_definition_payload(route, metadata).await
        } else if route == SEARCH_AUTOCOMPLETE_ROUTE {
            self.read_autocomplete_payload(route, metadata).await
        } else if route == QUERY_SQL_ROUTE {
            self.read_sql_payload(route, metadata).await
        } else if route == VFS_RESOLVE_ROUTE {
            self.read_vfs_resolve_payload(route, metadata).await
        } else if route == GRAPH_NEIGHBORS_ROUTE {
            self.read_graph_neighbors_payload(route, metadata).await
        } else if route == ANALYSIS_MARKDOWN_ROUTE {
            self.read_markdown_analysis_payload(route, metadata).await
        } else if route == ANALYSIS_CODE_AST_ROUTE {
            self.read_code_ast_analysis_payload(route, metadata).await
        } else if is_search_family_route(route) {
            self.read_search_family_payload(route, metadata).await
        } else {
            Err(Status::invalid_argument(format!(
                "unexpected routed Flight request: {route}"
            )))
        }
    }

    async fn read_repo_search_payload(
        &self,
        metadata: &tonic::metadata::MetadataMap,
    ) -> Result<FlightRoutePayload, Status> {
        let request = validate_repo_search_request_metadata(metadata)?;
        self.repo_search_provider
            .repo_search_batch(&request)
            .await
            .map_err(Status::internal)
            .and_then(FlightRoutePayload::try_new)
    }

    async fn read_attachment_search_payload(
        &self,
        route: &str,
        metadata: &tonic::metadata::MetadataMap,
    ) -> Result<FlightRoutePayload, Status> {
        let (query_text, limit, ext_filters, kind_filters, case_sensitive) =
            validate_attachment_search_request_metadata(metadata)?;
        let provider = self.attachment_search_provider.as_ref().ok_or_else(|| {
            Status::unimplemented(format!(
                "attachment-search Flight route `{route}` is not configured for this runtime host"
            ))
        })?;
        provider
            .attachment_search_batch(
                query_text.as_str(),
                limit,
                &ext_filters,
                &kind_filters,
                case_sensitive,
            )
            .await
            .map_err(Status::internal)
            .and_then(FlightRoutePayload::try_new)
    }

    async fn read_ast_search_payload(
        &self,
        route: &str,
        metadata: &tonic::metadata::MetadataMap,
    ) -> Result<FlightRoutePayload, Status> {
        let (query_text, limit, _intent, _repo_hint) = validate_search_request_metadata(metadata)?;
        let provider = self.ast_search_provider.as_ref().ok_or_else(|| {
            Status::unimplemented(format!(
                "AST-search Flight route `{route}` is not configured for this runtime host"
            ))
        })?;
        provider
            .ast_search_batch(query_text.as_str(), limit)
            .await
            .map_err(Status::internal)
            .and_then(FlightRoutePayload::try_new)
    }

    async fn read_definition_payload(
        &self,
        route: &str,
        metadata: &tonic::metadata::MetadataMap,
    ) -> Result<FlightRoutePayload, Status> {
        let (query_text, source_path, source_line) =
            validate_definition_request_metadata(metadata)?;
        let provider = self.definition_provider.as_ref().ok_or_else(|| {
            Status::unimplemented(format!(
                "definition Flight route `{route}` is not configured for this runtime host"
            ))
        })?;
        provider
            .definition_batch(query_text.as_str(), source_path.as_deref(), source_line)
            .await
            .and_then(|response| {
                FlightRoutePayload::try_with_app_metadata(response.batch, response.app_metadata)
            })
    }

    async fn read_autocomplete_payload(
        &self,
        route: &str,
        metadata: &tonic::metadata::MetadataMap,
    ) -> Result<FlightRoutePayload, Status> {
        let (prefix, limit) = validate_autocomplete_request_metadata(metadata)?;
        let provider = self.autocomplete_provider.as_ref().ok_or_else(|| {
            Status::unimplemented(format!(
                "autocomplete Flight route `{route}` is not configured for this runtime host"
            ))
        })?;
        provider
            .autocomplete_batch(prefix.as_str(), limit)
            .await
            .and_then(|response| {
                FlightRoutePayload::try_with_app_metadata(response.batch, response.app_metadata)
            })
    }

    async fn read_sql_payload(
        &self,
        route: &str,
        metadata: &tonic::metadata::MetadataMap,
    ) -> Result<FlightRoutePayload, Status> {
        let query_text = validate_sql_request_metadata(metadata)?;
        let provider = self.sql_provider.as_ref().ok_or_else(|| {
            Status::unimplemented(format!(
                "SQL Flight route `{route}` is not configured for this runtime host"
            ))
        })?;
        let response = provider
            .sql_query_batches(query_text.as_str())
            .await
            .map_err(Status::internal)?;
        FlightRoutePayload::from_batches_with_app_metadata(&response.batches, response.app_metadata)
    }

    async fn read_vfs_resolve_payload(
        &self,
        route: &str,
        metadata: &tonic::metadata::MetadataMap,
    ) -> Result<FlightRoutePayload, Status> {
        let path = validate_vfs_resolve_request_metadata(metadata)?;
        let provider = self.vfs_resolve_provider.as_ref().ok_or_else(|| {
            Status::unimplemented(format!(
                "VFS resolve Flight route `{route}` is not configured for this runtime host"
            ))
        })?;
        provider
            .resolve_vfs_navigation_batch(path.as_str())
            .await
            .and_then(|response| {
                FlightRoutePayload::try_with_app_metadata(response.batch, response.app_metadata)
            })
    }

    async fn read_graph_neighbors_payload(
        &self,
        route: &str,
        metadata: &tonic::metadata::MetadataMap,
    ) -> Result<FlightRoutePayload, Status> {
        let (node_id, direction, hops, limit) =
            validate_graph_neighbors_request_metadata(metadata)?;
        let provider = self.graph_neighbors_provider.as_ref().ok_or_else(|| {
            Status::unimplemented(format!(
                "graph-neighbors Flight route `{route}` is not configured for this runtime host"
            ))
        })?;
        provider
            .graph_neighbors_batch(node_id.as_str(), direction.as_str(), hops, limit)
            .await
            .and_then(|response| {
                FlightRoutePayload::try_with_app_metadata(response.batch, response.app_metadata)
            })
    }

    async fn read_markdown_analysis_payload(
        &self,
        route: &str,
        metadata: &tonic::metadata::MetadataMap,
    ) -> Result<FlightRoutePayload, Status> {
        let path = validate_markdown_analysis_request_metadata(metadata)?;
        let provider = self.markdown_analysis_provider.as_ref().ok_or_else(|| {
            Status::unimplemented(format!(
                "markdown analysis Flight route `{route}` is not configured for this runtime host"
            ))
        })?;
        provider
            .markdown_analysis_batch(path.as_str())
            .await
            .map_err(Status::internal)
            .and_then(|response| {
                FlightRoutePayload::try_with_app_metadata(response.batch, response.app_metadata)
            })
    }

    async fn read_code_ast_analysis_payload(
        &self,
        route: &str,
        metadata: &tonic::metadata::MetadataMap,
    ) -> Result<FlightRoutePayload, Status> {
        let (path, repo_id, line_hint) = validate_code_ast_analysis_request_metadata(metadata)?;
        let provider = self.code_ast_analysis_provider.as_ref().ok_or_else(|| {
            Status::unimplemented(format!(
                "code-AST analysis Flight route `{route}` is not configured for this runtime host"
            ))
        })?;
        provider
            .code_ast_analysis_batch(path.as_str(), repo_id.as_str(), line_hint)
            .await
            .map_err(Status::internal)
            .and_then(|response| {
                FlightRoutePayload::try_with_app_metadata(response.batch, response.app_metadata)
            })
    }

    async fn read_search_family_payload(
        &self,
        route: &str,
        metadata: &tonic::metadata::MetadataMap,
    ) -> Result<FlightRoutePayload, Status> {
        let (query_text, limit, intent, repo_hint) = validate_search_request_metadata(metadata)?;
        let provider = self.search_provider.as_ref().ok_or_else(|| {
            Status::unimplemented(format!(
                "search Flight route `{route}` is not configured for this runtime host"
            ))
        })?;
        provider
            .search_batch(
                route,
                query_text.as_str(),
                limit,
                intent.as_deref(),
                repo_hint.as_deref(),
            )
            .await
            .map_err(Status::internal)
            .and_then(|response| {
                FlightRoutePayload::try_with_app_metadata(response.batch, response.app_metadata)
            })
    }

    pub(super) async fn cached_route_payload(
        &self,
        route: &str,
        metadata: &tonic::metadata::MetadataMap,
        cache_key: &str,
    ) -> Result<Arc<FlightRoutePayload>, Status> {
        if let Some(cached) = self.route_payload_cache.get(cache_key).await {
            return Ok(cached);
        }
        let payload = self.read_route_payload(route, metadata).await?;
        Ok(self
            .route_payload_cache
            .insert(cache_key.to_string(), payload)
            .await)
    }
}
