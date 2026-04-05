use std::sync::Arc;

use async_trait::async_trait;
use tonic::Status;
use xiuxian_vector::{LanceDataType, LanceField, LanceRecordBatch, LanceSchema, LanceStringArray};
use xiuxian_wendao_runtime::transport::{
    AnalysisFlightRouteResponse, RefineDocFlightRouteProvider,
};

use crate::analyzers::{RefineEntityDocRequest, RefineEntityDocResponse};
use crate::gateway::studio::router::handlers::repo::command_service::run_refine_entity_doc;
use crate::gateway::studio::router::{GatewayState, StudioApiError};

#[derive(Clone)]
pub(crate) struct StudioRefineDocFlightRouteProvider {
    state: Arc<GatewayState>,
}

impl StudioRefineDocFlightRouteProvider {
    #[must_use]
    pub(crate) fn new(state: Arc<GatewayState>) -> Self {
        Self { state }
    }
}

impl std::fmt::Debug for StudioRefineDocFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioRefineDocFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl RefineDocFlightRouteProvider for StudioRefineDocFlightRouteProvider {
    async fn refine_doc_batch(
        &self,
        repo_id: &str,
        entity_id: &str,
        user_hints: Option<&str>,
    ) -> Result<AnalysisFlightRouteResponse, Status> {
        let response = run_refine_entity_doc(
            Arc::clone(&self.state),
            RefineEntityDocRequest {
                repo_id: repo_id.to_string(),
                entity_id: entity_id.to_string(),
                user_hints: user_hints.map(ToString::to_string),
            },
        )
        .await
        .map_err(studio_api_error_to_tonic_status)?;
        let batch = refine_doc_batch(&response).map_err(Status::internal)?;
        let metadata = refine_doc_metadata(&response).map_err(Status::internal)?;
        Ok(AnalysisFlightRouteResponse::new(batch).with_app_metadata(metadata))
    }
}

pub(crate) fn refine_doc_batch(
    response: &RefineEntityDocResponse,
) -> Result<LanceRecordBatch, String> {
    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new("repoId", LanceDataType::Utf8, false),
            LanceField::new("entityId", LanceDataType::Utf8, false),
            LanceField::new("refinedContent", LanceDataType::Utf8, false),
            LanceField::new("verificationState", LanceDataType::Utf8, false),
        ])),
        vec![
            Arc::new(LanceStringArray::from(vec![response.repo_id.as_str()])),
            Arc::new(LanceStringArray::from(vec![response.entity_id.as_str()])),
            Arc::new(LanceStringArray::from(vec![
                response.refined_content.as_str(),
            ])),
            Arc::new(LanceStringArray::from(vec![
                response.verification_state.as_str(),
            ])),
        ],
    )
    .map_err(|error| error.to_string())
}

pub(crate) fn refine_doc_metadata(response: &RefineEntityDocResponse) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&serde_json::json!({
        "repoId": response.repo_id,
        "entityId": response.entity_id,
        "refinedContent": response.refined_content,
        "verificationState": response.verification_state,
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

    fn demo_response() -> RefineEntityDocResponse {
        RefineEntityDocResponse {
            repo_id: "gateway-sync".to_string(),
            entity_id: "repo:gateway-sync:symbol:GatewaySyncPkg.solve".to_string(),
            refined_content: "## Refined Explanation\n\nUse `solve()`.".to_string(),
            verification_state: "verified".to_string(),
        }
    }

    #[test]
    fn refine_doc_batch_preserves_response_payload() {
        let batch = refine_doc_batch(&demo_response())
            .unwrap_or_else(|error| panic!("batch should build: {error}"));

        assert_eq!(batch.num_rows(), 1);
        let Some(entity_id_column) = batch.column_by_name("entityId") else {
            panic!("entityId column");
        };
        let Some(entity_ids) = entity_id_column.as_any().downcast_ref::<LanceStringArray>() else {
            panic!("entityId column type");
        };
        assert_eq!(
            entity_ids.value(0),
            "repo:gateway-sync:symbol:GatewaySyncPkg.solve"
        );

        let Some(refined_content_column) = batch.column_by_name("refinedContent") else {
            panic!("refinedContent column");
        };
        let Some(refined_content) = refined_content_column
            .as_any()
            .downcast_ref::<LanceStringArray>()
        else {
            panic!("refinedContent column type");
        };
        assert_eq!(
            refined_content.value(0),
            "## Refined Explanation\n\nUse `solve()`."
        );
    }

    #[test]
    fn refine_doc_metadata_preserves_summary_fields() {
        let metadata = refine_doc_metadata(&demo_response())
            .unwrap_or_else(|error| panic!("metadata should encode: {error}"));
        let payload: serde_json::Value = serde_json::from_slice(&metadata)
            .unwrap_or_else(|error| panic!("metadata should decode: {error}"));
        assert_eq!(payload["repoId"], "gateway-sync");
        assert_eq!(
            payload["entityId"],
            "repo:gateway-sync:symbol:GatewaySyncPkg.solve"
        );
        assert_eq!(payload["verificationState"], "verified");
    }
}
