use super::super::super::QianjiScheduler;
use super::super::super::types::{
    ConsensusCheckpointView, ConsensusOutcome, EXTERNAL_PROGRESS_TIMEOUT_MS,
};
use super::call_ctx::ConsensusCallCtx;
use crate::contracts::NodeStatus;
use crate::error::QianjiError;
use crate::telemetry::ConsensusStatus;
use tokio::time::Duration;

impl QianjiScheduler {
    pub(super) async fn handle_consensus_agreed(
        &self,
        call: &ConsensusCallCtx<'_>,
        agreed_hash: &str,
    ) -> Result<ConsensusOutcome, QianjiError> {
        self.emit_consensus_spike(
            call.session_id,
            call.node_id,
            ConsensusStatus::Agreed,
            Some(1.0),
            call.telemetry_target,
        );
        let agreed_output = self
            .read_agreed_output(
                call.manager,
                call.session_id,
                call.node_id,
                call.output_hash,
                agreed_hash,
                call.output_data,
            )
            .await?;
        Ok(ConsensusOutcome::Proceed(agreed_output))
    }

    pub(super) async fn handle_consensus_pending(
        &self,
        node_idx: petgraph::stable_graph::NodeIndex,
        checkpoint: &ConsensusCheckpointView<'_>,
        policy: &crate::consensus::ConsensusPolicy,
        call: &ConsensusCallCtx<'_>,
    ) -> Result<ConsensusOutcome, QianjiError> {
        self.emit_consensus_spike(
            call.session_id,
            call.node_id,
            ConsensusStatus::Pending,
            None,
            call.telemetry_target,
        );
        self.set_node_status(node_idx, NodeStatus::ConsensusPending)
            .await;
        self.save_checkpoint_if_needed(
            Some(call.session_id),
            checkpoint.redis_url,
            checkpoint.total_steps,
            checkpoint.active_branches,
            checkpoint.context,
        )
        .await;

        let wait_ms = if policy.timeout_ms == 0 {
            EXTERNAL_PROGRESS_TIMEOUT_MS
        } else {
            policy.timeout_ms
        };
        let wait_result = call
            .manager
            .wait_for_quorum(
                call.session_id,
                call.node_id,
                Duration::from_millis(wait_ms),
            )
            .await
            .map_err(|error| QianjiError::Execution(error.to_string()))?;
        if let Some(agreed_hash) = wait_result {
            let agreed_output = self
                .read_agreed_output(
                    call.manager,
                    call.session_id,
                    call.node_id,
                    call.output_hash,
                    &agreed_hash,
                    call.output_data,
                )
                .await?;
            return Ok(ConsensusOutcome::Proceed(agreed_output));
        }
        Ok(ConsensusOutcome::Suspend(checkpoint.context.clone()))
    }
}
