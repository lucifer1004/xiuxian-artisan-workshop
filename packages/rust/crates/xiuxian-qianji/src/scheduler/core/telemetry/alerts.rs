use crate::scheduler::core::QianjiScheduler;
use crate::telemetry::{SwarmEvent, unix_millis_now};

impl QianjiScheduler {
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
#[path = "../../../../tests/unit/scheduler/core/telemetry/alerts.rs"]
mod tests;
