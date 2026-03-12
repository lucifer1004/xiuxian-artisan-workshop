//! Session-scoped foreground gating.
//!
//! Guarantees that messages in the same logical session are processed serially,
//! while different sessions can execute concurrently.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::{Mutex, OwnedMutexGuard};

#[derive(Clone, Default)]
pub struct SessionGate {
    inner: Arc<StdMutex<HashMap<String, Arc<SessionGateEntry>>>>,
}

#[derive(Default)]
struct SessionGateEntry {
    lock: Arc<Mutex<()>>,
    permits: AtomicUsize,
}

pub struct SessionGuard {
    _lock_guard: OwnedMutexGuard<()>,
    _permit: SessionPermit,
}

struct SessionPermit {
    session_id: String,
    inner: Arc<StdMutex<HashMap<String, Arc<SessionGateEntry>>>>,
    entry: Arc<SessionGateEntry>,
}

impl Drop for SessionPermit {
    fn drop(&mut self) {
        let previous = self.entry.permits.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0, "session gate permit underflow");
        if previous != 1 {
            return;
        }

        let mut map = self
            .inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let should_remove = map
            .get(&self.session_id)
            .is_some_and(|current| Arc::ptr_eq(current, &self.entry))
            && self.entry.permits.load(Ordering::Acquire) == 0;
        if should_remove {
            map.remove(&self.session_id);
        }
    }
}

impl SessionGate {
    pub async fn acquire(&self, session_id: &str) -> SessionGuard {
        let entry = {
            let mut guard = self
                .inner
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard
                .entry(session_id.to_string())
                .or_insert_with(|| Arc::new(SessionGateEntry::default()))
                .clone()
        };

        entry.permits.fetch_add(1, Ordering::AcqRel);
        let lock_guard = entry.lock.clone().lock_owned().await;
        SessionGuard {
            _lock_guard: lock_guard,
            _permit: SessionPermit {
                session_id: session_id.to_string(),
                inner: Arc::clone(&self.inner),
                entry,
            },
        }
    }

    #[doc(hidden)]
    pub fn active_sessions(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }
}
