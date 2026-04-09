use super::QianjiScheduler;
use crate::telemetry::SwarmEvent;

mod alerts;
#[cfg(test)]
#[path = "../../../../tests/unit/scheduler/core/telemetry/node_transition.rs"]
mod node_transition;

impl QianjiScheduler {
    pub(in crate::scheduler::core) fn emit_event_non_blocking(&self, event: SwarmEvent) {
        let Some(emitter) = self.telemetry_emitter.clone() else {
            return;
        };
        std::mem::drop(tokio::spawn(async move {
            if let Err(error) = emitter.emit_pulse(event).await {
                log::debug!("scheduler telemetry emission skipped: {error}");
            }
        }));
    }
}
