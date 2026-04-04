use std::sync::Arc;

use async_trait::async_trait;
use tonic::Status;
use xiuxian_vector::{
    LanceDataType, LanceField, LanceRecordBatch, LanceSchema, LanceStringArray, LanceUInt64Array,
};
use xiuxian_wendao_runtime::transport::{
    AnalysisFlightRouteResponse, RepoProjectedPageIndexTreeFlightRouteProvider,
};

use crate::analyzers::RepoProjectedPageIndexTreeResult;
use crate::gateway::studio::router::handlers::repo::projected_service::run_repo_projected_page_index_tree;
use crate::gateway::studio::router::{GatewayState, StudioApiError};

#[derive(Clone)]
pub(crate) struct StudioRepoProjectedPageIndexTreeFlightRouteProvider {
    state: Arc<GatewayState>,
}

impl StudioRepoProjectedPageIndexTreeFlightRouteProvider {
    #[must_use]
    pub(crate) fn new(state: Arc<GatewayState>) -> Self {
        Self { state }
    }
}

impl std::fmt::Debug for StudioRepoProjectedPageIndexTreeFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioRepoProjectedPageIndexTreeFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl RepoProjectedPageIndexTreeFlightRouteProvider
    for StudioRepoProjectedPageIndexTreeFlightRouteProvider
{
    async fn repo_projected_page_index_tree_batch(
        &self,
        repo_id: &str,
        page_id: &str,
    ) -> Result<AnalysisFlightRouteResponse, Status> {
        let response = run_repo_projected_page_index_tree(
            Arc::clone(&self.state),
            crate::analyzers::RepoProjectedPageIndexTreeQuery {
                repo_id: repo_id.to_string(),
                page_id: page_id.to_string(),
            },
        )
        .await
        .map_err(studio_api_error_to_tonic_status)?;
        let batch = repo_projected_page_index_tree_batch(&response).map_err(Status::internal)?;
        let metadata =
            repo_projected_page_index_tree_metadata(&response).map_err(Status::internal)?;
        Ok(AnalysisFlightRouteResponse::new(batch).with_app_metadata(metadata))
    }
}

pub(crate) fn repo_projected_page_index_tree_batch(
    response: &RepoProjectedPageIndexTreeResult,
) -> Result<LanceRecordBatch, String> {
    let tree = response
        .tree
        .as_ref()
        .ok_or_else(|| "repo projected page-index tree payload is missing `tree`".to_string())?;
    let roots_json = serde_json::to_string(tree.roots.as_slice())
        .map_err(|error| format!("failed to encode projected page-index roots: {error}"))?;
    let root_count = u64::try_from(tree.root_count)
        .map_err(|error| format!("failed to represent projected page-index root count: {error}"))?;

    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new("repoId", LanceDataType::Utf8, false),
            LanceField::new("pageId", LanceDataType::Utf8, false),
            LanceField::new("path", LanceDataType::Utf8, false),
            LanceField::new("docId", LanceDataType::Utf8, false),
            LanceField::new("title", LanceDataType::Utf8, false),
            LanceField::new("rootCount", LanceDataType::UInt64, false),
            LanceField::new("rootsJson", LanceDataType::Utf8, false),
        ])),
        vec![
            Arc::new(LanceStringArray::from(vec![tree.repo_id.as_str()])),
            Arc::new(LanceStringArray::from(vec![tree.page_id.as_str()])),
            Arc::new(LanceStringArray::from(vec![tree.path.as_str()])),
            Arc::new(LanceStringArray::from(vec![tree.doc_id.as_str()])),
            Arc::new(LanceStringArray::from(vec![tree.title.as_str()])),
            Arc::new(LanceUInt64Array::from(vec![root_count])),
            Arc::new(LanceStringArray::from(vec![roots_json.as_str()])),
        ],
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn repo_projected_page_index_tree_metadata(
    response: &RepoProjectedPageIndexTreeResult,
) -> Result<Vec<u8>, String> {
    let tree = response
        .tree
        .as_ref()
        .ok_or_else(|| "repo projected page-index tree payload is missing `tree`".to_string())?;
    serde_json::to_vec(&serde_json::json!({
        "repoId": tree.repo_id,
        "pageId": tree.page_id,
        "path": tree.path,
        "docId": tree.doc_id,
        "title": tree.title,
        "rootCount": tree.root_count,
    }))
    .map_err(|error| error.to_string())
}

fn studio_api_error_to_tonic_status(error: StudioApiError) -> Status {
    match error.status() {
        axum::http::StatusCode::BAD_REQUEST => Status::invalid_argument(error.error.message),
        axum::http::StatusCode::NOT_FOUND => Status::not_found(error.error.message),
        axum::http::StatusCode::CONFLICT => Status::failed_precondition(error.error.message),
        _ => Status::internal(error.error.message),
    }
}

#[cfg(test)]
mod tests {
    use xiuxian_vector::{LanceArray, LanceStringArray};

    use super::*;
    use crate::analyzers::{ProjectedPageIndexNode, ProjectedPageIndexTree, ProjectionPageKind};

    fn demo_tree() -> ProjectedPageIndexTree {
        ProjectedPageIndexTree {
            repo_id: "gateway-sync".to_string(),
            page_id:
                "repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md"
                    .to_string(),
            kind: ProjectionPageKind::Reference,
            path: "docs/solve.md".to_string(),
            doc_id: "repo:gateway-sync:doc:docs/solve.md".to_string(),
            title: "solve".to_string(),
            root_count: 1,
            roots: vec![ProjectedPageIndexNode {
                node_id: "repo:gateway-sync:doc:docs/solve.md#root".to_string(),
                title: "solve".to_string(),
                level: 1,
                structural_path: vec!["solve".to_string()],
                line_range: (1, 3),
                token_count: 4,
                is_thinned: false,
                text: "solve docs".to_string(),
                summary: None,
                children: Vec::new(),
            }],
        }
    }

    #[test]
    fn projected_page_index_tree_batch_preserves_tree_payload() {
        let batch = repo_projected_page_index_tree_batch(&RepoProjectedPageIndexTreeResult {
            repo_id: "gateway-sync".to_string(),
            tree: Some(demo_tree()),
        })
        .expect("batch should build");

        assert_eq!(batch.num_rows(), 1);
        let page_ids = batch
            .column_by_name("pageId")
            .expect("pageId column")
            .as_any()
            .downcast_ref::<LanceStringArray>()
            .expect("pageId column type");
        assert_eq!(
            page_ids.value(0),
            "repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md"
        );

        let roots_json = batch
            .column_by_name("rootsJson")
            .expect("rootsJson column")
            .as_any()
            .downcast_ref::<LanceStringArray>()
            .expect("rootsJson column type");
        let roots: Vec<ProjectedPageIndexNode> =
            serde_json::from_str(roots_json.value(0)).expect("rootsJson should decode");
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].title, "solve");
    }

    #[test]
    fn projected_page_index_tree_metadata_preserves_summary_fields() {
        let metadata = repo_projected_page_index_tree_metadata(&RepoProjectedPageIndexTreeResult {
            repo_id: "gateway-sync".to_string(),
            tree: Some(demo_tree()),
        })
        .expect("metadata should encode");

        let payload: serde_json::Value =
            serde_json::from_slice(&metadata).expect("metadata should decode");
        assert_eq!(payload["repoId"], "gateway-sync");
        assert_eq!(payload["path"], "docs/solve.md");
        assert_eq!(payload["rootCount"], 1);
    }
}
