use super::super::super::orchestrator::SwarmEngine;
use super::super::super::types::{WorkerJoinSet, WorkerRuntimeConfig};
use super::super::super::{SwarmAgentConfig, SwarmAgentReport};
use crate::QianjiEngine;
use crate::error::QianjiError;
use crate::telemetry::{SwarmEvent, unix_millis_now};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use xiuxian_window::SessionWindow;

impl SwarmEngine {
    pub(in crate::swarm::engine) fn spawn_worker_task(
        &self,
        join_set: &mut WorkerJoinSet,
        identity: SwarmAgentConfig,
        context: serde_json::Value,
        runtime: WorkerRuntimeConfig,
        cancel_token: CancellationToken,
    ) {
        let engine = Arc::clone(&self.base_engine);
        join_set.spawn(Self::run_worker(
            engine,
            identity,
            context,
            runtime,
            cancel_token,
        ));
    }

    async fn run_worker(
        engine: Arc<QianjiEngine>,
        identity: SwarmAgentConfig,
        context: serde_json::Value,
        runtime: WorkerRuntimeConfig,
        cancel_token: CancellationToken,
    ) -> Result<SwarmAgentReport, QianjiError> {
        let session_id = runtime.session_id.clone();
        let role = identity.role_class.clone();
        let mut window = SessionWindow::new(
            format!("{session_id}:{}", identity.agent_id).as_str(),
            identity.window_size.max(32),
        );
        window.append_turn("system", "swarm_worker_boot", 0, Some(&session_id));

        let thread_id = format!("{:?}", std::thread::current().id());
        log::info!(
            "[THREAD_ID={thread_id}] [AGENT_ID={}] swarm worker started",
            identity.agent_id
        );
        Self::emit_pulse_event(
            runtime.pulse_emitter.as_ref(),
            SwarmEvent::SwarmHeartbeat {
                session_id: Some(session_id.clone()),
                cluster_id: runtime.cluster_id.clone(),
                agent_id: Some(identity.agent_id.clone()),
                role_class: role.clone(),
                cpu_percent: None,
                memory_bytes: None,
                timestamp_ms: unix_millis_now(),
            },
        );

        let scheduler = Self::build_worker_scheduler(&engine, &identity, &runtime);
        let (stop_tx, responder_handle) = Self::start_remote_responder(
            Arc::clone(&scheduler),
            role.clone(),
            identity.agent_id.clone(),
            runtime.remote_enabled,
            runtime.poll_interval_ms,
        );

        let run_future =
            scheduler.run_with_checkpoint(context, Some(session_id.clone()), runtime.redis_url);
        tokio::pin!(run_future);
        let run_result = tokio::select! {
            () = cancel_token.cancelled() => Err(QianjiError::Aborted(format!(
                "swarm worker '{}' cancelled by global fault broadcast",
                identity.agent_id
            ))),
            result = &mut run_future => result,
        };
        Self::stop_remote_responder(stop_tx, responder_handle).await;

        Self::build_worker_report(identity, role, session_id.as_str(), &mut window, run_result)
    }
}
