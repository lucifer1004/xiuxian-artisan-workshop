use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use tokio::sync::{Mutex, OwnedMutexGuard};

use super::config::{SessionGateBackendMode, SessionGateRuntimeConfig};
use super::valkey::{DistributedLeaseGuard, ValkeySessionGateBackend};

#[derive(Clone)]
pub struct SessionGate {
    pub(super) inner: Arc<StdMutex<HashMap<String, Arc<SessionGateEntry>>>>,
    pub(super) backend: SessionGateBackend,
}

#[derive(Default)]
pub(super) struct SessionGateEntry {
    pub(super) lock: Arc<Mutex<()>>,
    pub(super) permits: AtomicUsize,
}

pub struct SessionGuard {
    pub(super) _distributed_lease: Option<DistributedLeaseGuard>,
    pub(super) _lock_guard: OwnedMutexGuard<()>,
    pub(super) _permit: SessionPermit,
}

pub(super) struct SessionPermit {
    pub(super) session_id: String,
    pub(super) inner: Arc<StdMutex<HashMap<String, Arc<SessionGateEntry>>>>,
    pub(super) entry: Arc<SessionGateEntry>,
}

#[derive(Clone)]
pub(super) enum SessionGateBackend {
    Memory,
    Valkey(Arc<ValkeySessionGateBackend>),
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
    pub(crate) fn from_env() -> Result<Self> {
        let runtime_config = SessionGateRuntimeConfig::from_env()?;
        let backend = match runtime_config.backend_mode {
            SessionGateBackendMode::Auto | SessionGateBackendMode::Memory => {
                SessionGateBackend::Memory
            }
            SessionGateBackendMode::Valkey => {
                let backend = runtime_config
                    .valkey_url
                    .as_deref()
                    .map(|valkey_url| {
                        ValkeySessionGateBackend::new(
                            valkey_url,
                            runtime_config.key_prefix.as_str(),
                            runtime_config.lease_ttl_secs,
                            runtime_config.acquire_timeout_secs,
                        )
                    })
                    .transpose()?
                    .map(Arc::new);
                match backend {
                    Some(backend) => SessionGateBackend::Valkey(backend),
                    None => SessionGateBackend::Memory,
                }
            }
        };

        Ok(Self {
            inner: Arc::new(StdMutex::new(HashMap::new())),
            backend,
        })
    }

    pub(crate) fn backend_name(&self) -> &'static str {
        match self.backend {
            SessionGateBackend::Memory => "memory",
            SessionGateBackend::Valkey(_) => "valkey",
        }
    }

    pub(crate) async fn acquire(&self, session_id: &str) -> Result<SessionGuard> {
        let distributed_lease = match &self.backend {
            SessionGateBackend::Memory => None,
            SessionGateBackend::Valkey(backend) => Some(backend.acquire_lease(session_id).await?),
        };

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
        Ok(SessionGuard {
            _distributed_lease: distributed_lease,
            _lock_guard: lock_guard,
            _permit: SessionPermit {
                session_id: session_id.to_string(),
                inner: Arc::clone(&self.inner),
                entry,
            },
        })
    }
}
