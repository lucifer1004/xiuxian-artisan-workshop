use std::collections::HashMap;
use std::sync::{Arc, Condvar, LazyLock, Mutex};
use std::time::Duration;

pub(in crate::llm::vision::deepseek::native) type SharedCoalescedResult =
    Result<Option<Arc<str>>, Arc<str>>;

pub(in crate::llm::vision::deepseek::native) enum CoalesceAcquire {
    Leader(CoalesceLeaderPermit),
    Follower(CoalesceFollower),
}

pub(in crate::llm::vision::deepseek::native) struct CoalesceLeaderPermit {
    key: String,
}

pub(in crate::llm::vision::deepseek::native) struct CoalesceFollower {
    state: Arc<(Mutex<CoalesceState>, Condvar)>,
}

#[derive(Default)]
struct CoalesceState {
    result: Option<SharedCoalescedResult>,
    followers: usize,
}

struct GlobalCoalescer {
    inflight: HashMap<String, Arc<(Mutex<CoalesceState>, Condvar)>>,
}

static COALESCER: LazyLock<Mutex<GlobalCoalescer>> = LazyLock::new(|| {
    Mutex::new(GlobalCoalescer {
        inflight: HashMap::new(),
    })
});

pub(in crate::llm::vision::deepseek::native) fn acquire(
    key: &str,
    _stale_timeout: Duration,
) -> CoalesceAcquire {
    let mut guard = COALESCER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(state) = guard.inflight.get(key) {
        let mut state_guard = state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state_guard.followers += 1;
        CoalesceAcquire::Follower(CoalesceFollower {
            state: Arc::clone(state),
        })
    } else {
        let state = Arc::new((Mutex::new(CoalesceState::default()), Condvar::new()));
        guard.inflight.insert(key.to_string(), state);
        CoalesceAcquire::Leader(CoalesceLeaderPermit {
            key: key.to_string(),
        })
    }
}

impl CoalesceLeaderPermit {
    pub(in crate::llm::vision::deepseek::native) fn complete(self, result: SharedCoalescedResult) {
        let mut guard = COALESCER
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(state_arc) = guard.inflight.remove(&self.key) {
            let mut state = state_arc
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state.result = Some(result);
            state_arc.1.notify_all();
        }
    }

    pub(in crate::llm::vision::deepseek::native) fn follower_count(&self) -> usize {
        let guard = COALESCER
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.inflight.get(&self.key).map_or(0, |s| {
            let guard =
                s.0.lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard.followers
        })
    }
}

impl CoalesceFollower {
    pub(in crate::llm::vision::deepseek::native) fn wait(
        self,
        timeout: Duration,
    ) -> Option<SharedCoalescedResult> {
        let (lock, cvar) = &*self.state;
        let mut state = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let started = std::time::Instant::now();
        while state.result.is_none() {
            let elapsed = started.elapsed();
            if elapsed >= timeout {
                return None;
            }
            // SAFETY: We verified elapsed < timeout above, so checked_sub will always succeed
            let remaining = timeout.saturating_sub(elapsed);
            let (new_state, wait_result) = cvar
                .wait_timeout(state, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state = new_state;
            if wait_result.timed_out() {
                break;
            }
        }
        state.result.clone()
    }
}
