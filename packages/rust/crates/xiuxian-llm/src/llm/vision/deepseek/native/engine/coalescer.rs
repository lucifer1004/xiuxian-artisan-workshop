use std::sync::{Arc, LazyLock, Mutex};

use super::core::DeepseekEngine;

pub(in crate::llm::vision::deepseek::native) type CachedEngineEntry =
    Result<Arc<DeepseekEngine>, Arc<str>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::llm::vision::deepseek::native) enum EngineSlot {
    Primary,
    PrimaryCpuFallback,
    Dots,
    DotsCpuFallback,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::llm::vision::deepseek::native) enum EngineRegistryEntryState {
    Empty,
    Cached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::llm::vision::deepseek::native) struct EngineRegistrySnapshot {
    pub(in crate::llm::vision::deepseek::native) primary: EngineRegistryEntryState,
    pub(in crate::llm::vision::deepseek::native) primary_cpu_fallback: EngineRegistryEntryState,
    pub(in crate::llm::vision::deepseek::native) dots: EngineRegistryEntryState,
    pub(in crate::llm::vision::deepseek::native) dots_cpu_fallback: EngineRegistryEntryState,
}

#[derive(Default)]
struct EngineRegistry {
    primary: Option<CachedEngineEntry>,
    primary_cpu_fallback: Option<CachedEngineEntry>,
    dots: Option<CachedEngineEntry>,
    dots_cpu_fallback: Option<CachedEngineEntry>,
}

static ENGINE_REGISTRY: LazyLock<Mutex<EngineRegistry>> =
    LazyLock::new(|| Mutex::new(EngineRegistry::default()));

impl EngineRegistryEntryState {
    fn from_present(present: bool) -> Self {
        if present { Self::Cached } else { Self::Empty }
    }
}

impl EngineRegistry {
    fn slot_mut(&mut self, slot: EngineSlot) -> &mut Option<CachedEngineEntry> {
        match slot {
            EngineSlot::Primary => &mut self.primary,
            EngineSlot::PrimaryCpuFallback => &mut self.primary_cpu_fallback,
            EngineSlot::Dots => &mut self.dots,
            EngineSlot::DotsCpuFallback => &mut self.dots_cpu_fallback,
        }
    }
}

pub(in crate::llm::vision::deepseek::native) fn get_or_init_cached_engine<F>(
    slot: EngineSlot,
    init: F,
) -> CachedEngineEntry
where
    F: FnOnce() -> CachedEngineEntry,
{
    let mut guard = ENGINE_REGISTRY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let entry = guard.slot_mut(slot);
    if let Some(cached) = entry.as_ref() {
        return clone_cached_engine_entry(cached);
    }

    let loaded = init();
    *entry = Some(clone_cached_engine_entry(&loaded));
    loaded
}

pub(in crate::llm::vision::deepseek::native) fn snapshot_registry_for_tests()
-> EngineRegistrySnapshot {
    let guard = ENGINE_REGISTRY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    EngineRegistrySnapshot {
        primary: EngineRegistryEntryState::from_present(guard.primary.is_some()),
        primary_cpu_fallback: EngineRegistryEntryState::from_present(
            guard.primary_cpu_fallback.is_some(),
        ),
        dots: EngineRegistryEntryState::from_present(guard.dots.is_some()),
        dots_cpu_fallback: EngineRegistryEntryState::from_present(
            guard.dots_cpu_fallback.is_some(),
        ),
    }
}

pub(in crate::llm::vision::deepseek::native) fn clear_registry_for_tests() {
    let mut guard = ENGINE_REGISTRY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard = EngineRegistry::default();
}

pub(in crate::llm::vision::deepseek::native) fn seed_failure_for_tests(
    slot: EngineSlot,
    message: &str,
) {
    let mut guard = ENGINE_REGISTRY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *guard.slot_mut(slot) = Some(Err(Arc::from(message)));
}

fn clone_cached_engine_entry(entry: &CachedEngineEntry) -> CachedEngineEntry {
    match entry {
        Ok(engine) => Ok(Arc::clone(engine)),
        Err(error) => Err(Arc::clone(error)),
    }
}
