use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use tokio::sync::watch;

#[derive(Clone)]
struct SessionInterruptState {
    sender: watch::Sender<u64>,
    active_generations: usize,
}

/// In-memory foreground interrupt controller keyed by Discord session id.
#[derive(Clone, Default)]
pub(in crate::channels::discord::runtime) struct ForegroundInterruptController {
    channels: Arc<Mutex<HashMap<String, SessionInterruptState>>>,
}

impl ForegroundInterruptController {
    pub(in crate::channels::discord::runtime) fn begin_generation(
        &self,
        session_id: &str,
    ) -> watch::Receiver<u64> {
        let mut guard = self.channels.lock().unwrap_or_else(PoisonError::into_inner);
        let state = guard
            .entry(session_id.to_string())
            .or_insert_with(|| SessionInterruptState {
                sender: watch::channel(0u64).0,
                active_generations: 0,
            });
        state.active_generations = state.active_generations.saturating_add(1);
        state.sender.subscribe()
    }

    pub(in crate::channels::discord::runtime) fn end_generation(&self, session_id: &str) {
        let mut guard = self.channels.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(state) = guard.get_mut(session_id) {
            state.active_generations = state.active_generations.saturating_sub(1);
        }
    }

    pub(in crate::channels::discord::runtime) fn interrupt(&self, session_id: &str) -> bool {
        let state = {
            let guard = self.channels.lock().unwrap_or_else(PoisonError::into_inner);
            guard.get(session_id).cloned()
        };
        let Some(state) = state else {
            return false;
        };
        if state.active_generations == 0 {
            return false;
        }

        let next_generation = (*state.sender.borrow()).saturating_add(1);
        let _ = state.sender.send_replace(next_generation);
        true
    }
}
