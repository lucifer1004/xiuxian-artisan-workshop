use std::sync::Arc;

use async_trait::async_trait;
use xiuxian_vector::{LanceDataType, LanceField, LanceRecordBatch, LanceSchema, LanceStringArray};
use xiuxian_wendao_runtime::transport::{AnalysisFlightRouteResponse, RepoSyncFlightRouteProvider};

use crate::analyzers::{RepoSyncMode, RepoSyncResult};
use crate::gateway::studio::router::GatewayState;
use crate::gateway::studio::router::handlers::repo::command_service::run_repo_sync;

#[derive(Clone)]
pub(crate) struct StudioRepoSyncFlightRouteProvider {
    state: Arc<GatewayState>,
}

impl StudioRepoSyncFlightRouteProvider {
    #[must_use]
    pub(crate) fn new(state: Arc<GatewayState>) -> Self {
        Self { state }
    }
}

impl std::fmt::Debug for StudioRepoSyncFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioRepoSyncFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl RepoSyncFlightRouteProvider for StudioRepoSyncFlightRouteProvider {
    async fn repo_sync_batch(
        &self,
        repo_id: &str,
        mode: &str,
    ) -> Result<AnalysisFlightRouteResponse, String> {
        let response = run_repo_sync(
            Arc::clone(&self.state),
            repo_id.to_string(),
            parse_repo_sync_mode(mode)?,
        )
        .await
        .map_err(map_studio_api_error)?;
        let batch = build_repo_sync_flight_batch(&response)?;
        let metadata = serde_json::to_vec(&response).map_err(|error| error.to_string())?;
        Ok(AnalysisFlightRouteResponse::new(batch).with_app_metadata(metadata))
    }
}

fn parse_repo_sync_mode(mode: &str) -> Result<RepoSyncMode, String> {
    match mode {
        "ensure" => Ok(RepoSyncMode::Ensure),
        "refresh" => Ok(RepoSyncMode::Refresh),
        "status" => Ok(RepoSyncMode::Status),
        other => Err(format!("unsupported repo sync mode `{other}`")),
    }
}

fn build_repo_sync_flight_batch(response: &RepoSyncResult) -> Result<LanceRecordBatch, String> {
    let mode = json_string(&response.mode)?;
    let source_kind = json_string(&response.source_kind)?;
    let refresh = json_string(&response.refresh)?;
    let mirror_state = json_string(&response.mirror_state)?;
    let checkout_state = json_string(&response.checkout_state)?;
    let health_state = json_string(&response.health_state)?;
    let staleness_state = json_string(&response.staleness_state)?;
    let drift_state = json_string(&response.drift_state)?;
    let status_summary_json =
        serde_json::to_string(&response.status_summary).map_err(|error| error.to_string())?;

    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new("repoId", LanceDataType::Utf8, false),
            LanceField::new("mode", LanceDataType::Utf8, false),
            LanceField::new("sourceKind", LanceDataType::Utf8, false),
            LanceField::new("refresh", LanceDataType::Utf8, false),
            LanceField::new("mirrorState", LanceDataType::Utf8, false),
            LanceField::new("checkoutState", LanceDataType::Utf8, false),
            LanceField::new("revision", LanceDataType::Utf8, true),
            LanceField::new("checkoutPath", LanceDataType::Utf8, false),
            LanceField::new("mirrorPath", LanceDataType::Utf8, true),
            LanceField::new("checkedAt", LanceDataType::Utf8, false),
            LanceField::new("lastFetchedAt", LanceDataType::Utf8, true),
            LanceField::new("upstreamUrl", LanceDataType::Utf8, true),
            LanceField::new("healthState", LanceDataType::Utf8, false),
            LanceField::new("stalenessState", LanceDataType::Utf8, false),
            LanceField::new("driftState", LanceDataType::Utf8, false),
            LanceField::new("statusSummaryJson", LanceDataType::Utf8, false),
        ])),
        vec![
            Arc::new(LanceStringArray::from(vec![response.repo_id.clone()])),
            Arc::new(LanceStringArray::from(vec![mode])),
            Arc::new(LanceStringArray::from(vec![source_kind])),
            Arc::new(LanceStringArray::from(vec![refresh])),
            Arc::new(LanceStringArray::from(vec![mirror_state])),
            Arc::new(LanceStringArray::from(vec![checkout_state])),
            Arc::new(LanceStringArray::from(vec![response.revision.clone()])),
            Arc::new(LanceStringArray::from(vec![response.checkout_path.clone()])),
            Arc::new(LanceStringArray::from(vec![response.mirror_path.clone()])),
            Arc::new(LanceStringArray::from(vec![response.checked_at.clone()])),
            Arc::new(LanceStringArray::from(vec![
                response.last_fetched_at.clone(),
            ])),
            Arc::new(LanceStringArray::from(vec![response.upstream_url.clone()])),
            Arc::new(LanceStringArray::from(vec![health_state])),
            Arc::new(LanceStringArray::from(vec![staleness_state])),
            Arc::new(LanceStringArray::from(vec![drift_state])),
            Arc::new(LanceStringArray::from(vec![status_summary_json])),
        ],
    )
    .map_err(|error| error.to_string())
}

fn json_string<T>(value: &T) -> Result<String, String>
where
    T: serde::Serialize,
{
    serde_json::to_value(value)
        .map_err(|error| error.to_string())?
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| "repo sync enum metadata must serialize as string".to_string())
}

fn map_studio_api_error(error: crate::gateway::studio::router::StudioApiError) -> String {
    error
        .error
        .details
        .clone()
        .unwrap_or_else(|| format!("{}: {}", error.code(), error.error.message))
}

#[cfg(test)]
mod tests {
    use crate::analyzers::{
        RepoSourceKind, RepoSyncDriftState, RepoSyncFreshnessSummary, RepoSyncHealthState,
        RepoSyncLifecycleSummary, RepoSyncResult, RepoSyncRevisionSummary, RepoSyncStalenessState,
        RepoSyncState, RepoSyncStatusSummary,
    };
    use xiuxian_vector::LanceArray;

    use super::*;

    fn sample_sync_result() -> RepoSyncResult {
        RepoSyncResult {
            repo_id: "gateway-sync".to_string(),
            mode: RepoSyncMode::Status,
            source_kind: RepoSourceKind::ManagedRemote,
            refresh: crate::analyzers::config::RepositoryRefreshPolicy::Fetch,
            mirror_state: RepoSyncState::Validated,
            checkout_state: RepoSyncState::Reused,
            checkout_path: "/tmp/gateway-sync".to_string(),
            mirror_path: Some("/tmp/gateway-sync.mirror".to_string()),
            checked_at: "2026-04-03T19:15:00Z".to_string(),
            last_fetched_at: Some("2026-04-03T19:10:00Z".to_string()),
            mirror_revision: Some("mirror:123".to_string()),
            tracking_revision: Some("tracking:123".to_string()),
            upstream_url: Some("https://example.com/repo.git".to_string()),
            drift_state: RepoSyncDriftState::InSync,
            health_state: RepoSyncHealthState::Healthy,
            staleness_state: RepoSyncStalenessState::Fresh,
            status_summary: RepoSyncStatusSummary {
                lifecycle: RepoSyncLifecycleSummary {
                    source_kind: RepoSourceKind::ManagedRemote,
                    mirror_state: RepoSyncState::Validated,
                    checkout_state: RepoSyncState::Reused,
                    mirror_ready: true,
                    checkout_ready: true,
                },
                freshness: RepoSyncFreshnessSummary {
                    checked_at: "2026-04-03T19:15:00Z".to_string(),
                    last_fetched_at: Some("2026-04-03T19:10:00Z".to_string()),
                    staleness_state: RepoSyncStalenessState::Fresh,
                },
                revisions: RepoSyncRevisionSummary {
                    checkout_revision: Some("rev:123".to_string()),
                    mirror_revision: Some("mirror:123".to_string()),
                    tracking_revision: Some("tracking:123".to_string()),
                    aligned_with_mirror: true,
                },
                health_state: RepoSyncHealthState::Healthy,
                drift_state: RepoSyncDriftState::InSync,
                attention_required: false,
            },
            revision: Some("rev:123".to_string()),
        }
    }

    #[test]
    fn repo_sync_flight_batch_preserves_summary_fields() {
        let batch = build_repo_sync_flight_batch(&sample_sync_result())
            .expect("repo sync batch should build");

        assert_eq!(batch.num_rows(), 1);
        let mode = batch
            .column_by_name("mode")
            .expect("mode column")
            .as_any()
            .downcast_ref::<LanceStringArray>()
            .expect("mode should be utf8");
        assert_eq!(mode.value(0), "status");

        let health_state = batch
            .column_by_name("healthState")
            .expect("healthState column")
            .as_any()
            .downcast_ref::<LanceStringArray>()
            .expect("healthState should be utf8");
        assert_eq!(health_state.value(0), "healthy");
    }

    #[test]
    fn repo_sync_flight_metadata_preserves_full_payload() {
        let metadata = serde_json::to_vec(&sample_sync_result()).expect("metadata should encode");
        let payload: serde_json::Value =
            serde_json::from_slice(&metadata).expect("metadata should decode");
        assert_eq!(payload["repo_id"], "gateway-sync");
        assert_eq!(payload["mode"], "status");
        assert_eq!(payload["health_state"], "healthy");
    }
}
