use crate::scheduler::core::QianjiScheduler;
#[cfg(test)]
use crate::telemetry::ConsensusStatus;
use crate::telemetry::{SwarmEvent, unix_millis_now};

impl QianjiScheduler {
    #[cfg(test)]
    pub(in crate::scheduler::core) fn emit_consensus_spike(
        &self,
        session_id: &str,
        node_id: &str,
        status: ConsensusStatus,
        progress: Option<f32>,
        target: Option<f32>,
    ) {
        self.emit_event_non_blocking(SwarmEvent::ConsensusSpike {
            session_id: session_id.to_string(),
            node_id: node_id.to_string(),
            status,
            progress,
            target,
            timestamp_ms: unix_millis_now(),
        });
    }

    pub(in crate::scheduler::core) fn emit_affinity_alert(
        &self,
        node_id: String,
        required_role: &str,
        session_id: Option<&str>,
    ) {
        self.emit_event_non_blocking(SwarmEvent::AffinityAlert {
            session_id: session_id.map(std::string::ToString::to_string),
            node_id,
            required_role: required_role.to_string(),
            proxy_agent_id: self.execution_identity.agent_id.clone(),
            proxy_role: self.execution_identity.role_class.clone(),
            timestamp_ms: unix_millis_now(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::QianjiEngine;

    #[test]
    fn emit_consensus_spike_is_callable_without_emitter() {
        let scheduler = QianjiScheduler::new(QianjiEngine::default());
        scheduler.emit_consensus_spike(
            "session-1",
            "node-1",
            ConsensusStatus::Failed,
            Some(0.5),
            Some(0.8),
        );
    }
}
