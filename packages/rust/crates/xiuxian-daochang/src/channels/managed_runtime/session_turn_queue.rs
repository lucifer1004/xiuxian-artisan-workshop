use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex as StdMutex, PoisonError};

use tokio::sync::oneshot;

#[derive(Clone, Default)]
pub(crate) struct SessionTurnQueue {
    inner: Arc<StdMutex<HashMap<String, SessionTurnQueueEntry>>>,
}

#[derive(Default)]
struct SessionTurnQueueEntry {
    running: bool,
    permits: usize,
    waiters: VecDeque<oneshot::Sender<()>>,
}

pub(crate) struct SessionTurnTicket {
    inner: Arc<StdMutex<HashMap<String, SessionTurnQueueEntry>>>,
    session_id: String,
    state: SessionTurnTicketState,
}

enum SessionTurnTicketState {
    Immediate,
    Queued { ready_rx: oneshot::Receiver<()> },
    Consumed,
}

pub(crate) struct SessionTurnGuard {
    inner: Arc<StdMutex<HashMap<String, SessionTurnQueueEntry>>>,
    session_id: String,
}

impl SessionTurnQueue {
    pub(crate) fn enqueue(&self, session_id: &str) -> SessionTurnTicket {
        let mut guard = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        let entry = guard.entry(session_id.to_string()).or_default();
        entry.permits = entry.permits.saturating_add(1);
        let state = if entry.running {
            let (ready_tx, ready_rx) = oneshot::channel();
            entry.waiters.push_back(ready_tx);
            SessionTurnTicketState::Queued { ready_rx }
        } else {
            entry.running = true;
            SessionTurnTicketState::Immediate
        };
        SessionTurnTicket {
            inner: Arc::clone(&self.inner),
            session_id: session_id.to_string(),
            state,
        }
    }
}

impl SessionTurnTicket {
    pub(crate) async fn wait_turn(mut self) -> SessionTurnGuard {
        let state = std::mem::replace(&mut self.state, SessionTurnTicketState::Consumed);
        if let SessionTurnTicketState::Queued { ready_rx } = state {
            let _ = ready_rx.await;
        }
        SessionTurnGuard {
            inner: Arc::clone(&self.inner),
            session_id: self.session_id.clone(),
        }
    }
}

impl Drop for SessionTurnTicket {
    fn drop(&mut self) {
        match self.state {
            SessionTurnTicketState::Immediate => {
                release_active_turn(&self.inner, &self.session_id);
            }
            SessionTurnTicketState::Queued { .. } => {
                cancel_waiting_turn(&self.inner, &self.session_id);
            }
            SessionTurnTicketState::Consumed => {}
        }
    }
}

impl Drop for SessionTurnGuard {
    fn drop(&mut self) {
        release_active_turn(&self.inner, &self.session_id);
    }
}

fn cancel_waiting_turn(
    inner: &Arc<StdMutex<HashMap<String, SessionTurnQueueEntry>>>,
    session_id: &str,
) {
    let mut guard = inner.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(entry) = guard.get_mut(session_id) else {
        return;
    };
    entry.permits = entry.permits.saturating_sub(1);
    let remove_entry = !entry.running && entry.permits == 0;
    if remove_entry {
        guard.remove(session_id);
    }
}

fn release_active_turn(
    inner: &Arc<StdMutex<HashMap<String, SessionTurnQueueEntry>>>,
    session_id: &str,
) {
    let mut guard = inner.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(entry) = guard.get_mut(session_id) else {
        return;
    };
    entry.permits = entry.permits.saturating_sub(1);

    while let Some(waiter) = entry.waiters.pop_front() {
        if waiter.send(()).is_ok() {
            return;
        }
    }

    entry.running = false;
    if entry.permits == 0 {
        guard.remove(session_id);
    }
}
