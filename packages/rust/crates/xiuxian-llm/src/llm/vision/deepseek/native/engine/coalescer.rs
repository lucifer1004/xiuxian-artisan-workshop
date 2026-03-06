use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

pub(super) type SharedCoalescedResult = Result<Option<Arc<str>>, Arc<str>>;

static INFLIGHT: OnceLock<Mutex<HashMap<String, Arc<InflightEntry>>>> = OnceLock::new();

pub(super) enum CoalesceAcquire {
    Leader(CoalesceLeaderPermit),
    Follower(CoalesceFollowerPermit),
}

pub(super) struct CoalesceLeaderPermit {
    key: String,
    entry: Arc<InflightEntry>,
}

pub(super) struct CoalesceFollowerPermit {
    entry: Arc<InflightEntry>,
}

struct InflightEntry {
    state: Mutex<EntryState>,
    ready: Condvar,
}

struct EntryState {
    started_at: Instant,
    result: Option<SharedCoalescedResult>,
    follower_count: usize,
}

impl InflightEntry {
    fn new() -> Self {
        Self {
            state: Mutex::new(EntryState {
                started_at: Instant::now(),
                result: None,
                follower_count: 0,
            }),
            ready: Condvar::new(),
        }
    }

    fn is_stale(&self, stale_after: Duration) -> bool {
        let guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.result.is_none() && guard.started_at.elapsed() >= stale_after
    }

    fn add_follower(&self) {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.follower_count = guard.follower_count.saturating_add(1);
    }

    fn follower_count(&self) -> usize {
        let guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.follower_count
    }

    fn wait_result(&self, timeout: Duration) -> Option<SharedCoalescedResult> {
        let start = Instant::now();
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            if let Some(result) = guard.result.as_ref() {
                return Some(clone_shared_result(result));
            }
            let elapsed = start.elapsed();
            if elapsed >= timeout {
                return None;
            }
            let remaining = timeout.saturating_sub(elapsed);
            let (next_guard, _) = self
                .ready
                .wait_timeout(guard, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard = next_guard;
        }
    }

    fn complete(&self, result: SharedCoalescedResult) {
        let mut guard = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.result = Some(result);
        self.ready.notify_all();
    }
}

pub(super) fn acquire(cache_key: &str, stale_after: Duration) -> CoalesceAcquire {
    let map = INFLIGHT.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = map
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if let Some(entry) = guard.get(cache_key).cloned() {
        if entry.is_stale(stale_after) {
            tracing::warn!(
                event = "llm.vision.deepseek.infer.coalesce_stale_reap",
                stale_after_ms = stale_after.as_millis(),
                key_len = cache_key.len(),
                "DeepSeek OCR coalescer reaped stale in-flight entry"
            );
            guard.remove(cache_key);
        } else {
            entry.add_follower();
            return CoalesceAcquire::Follower(CoalesceFollowerPermit { entry });
        }
    }

    let entry = Arc::new(InflightEntry::new());
    guard.insert(cache_key.to_string(), Arc::clone(&entry));
    CoalesceAcquire::Leader(CoalesceLeaderPermit {
        key: cache_key.to_string(),
        entry,
    })
}

impl CoalesceLeaderPermit {
    pub(super) fn follower_count(&self) -> usize {
        self.entry.follower_count()
    }

    pub(super) fn complete(self, result: SharedCoalescedResult) {
        self.entry.complete(result);
        let map = INFLIGHT.get_or_init(|| Mutex::new(HashMap::new()));
        let mut guard = map
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(current) = guard.get(self.key.as_str())
            && Arc::ptr_eq(current, &self.entry)
        {
            guard.remove(self.key.as_str());
        }
    }
}

impl CoalesceFollowerPermit {
    pub(super) fn wait(self, timeout: Duration) -> Option<SharedCoalescedResult> {
        self.entry.wait_result(timeout)
    }
}

fn clone_shared_result(result: &SharedCoalescedResult) -> SharedCoalescedResult {
    match result {
        Ok(Some(value)) => Ok(Some(Arc::clone(value))),
        Ok(None) => Ok(None),
        Err(error) => Err(Arc::clone(error)),
    }
}
