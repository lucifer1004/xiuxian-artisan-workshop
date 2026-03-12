mod batch_lane;
mod cache_io;
mod coalescer;
mod core;
mod image_decode;
mod lifecycle;
mod loader;
mod registry;
mod retry;
mod telemetry;

pub(in crate::llm::vision::deepseek::native) use self::batch_lane::{
    clear_for_tests as clear_batch_lane_for_tests,
    force_draining_for_tests as force_batch_lane_draining_for_tests,
    snapshot_for_tests as batch_lane_snapshot_for_tests,
};
pub(in crate::llm::vision::deepseek::native) use self::coalescer::{
    clear_for_tests as clear_coalescer_for_tests,
    drop_leader_without_completion_for_tests as drop_coalescer_leader_without_completion_for_tests,
    len_for_tests as coalescer_len_for_tests,
    seed_entry_for_tests as seed_coalescer_entry_for_tests,
};
pub(in crate::llm::vision::deepseek::native) use self::lifecycle::{
    clear_cpu_fallback_flags_for_tests, force_dots_cpu_fallback_for_tests,
    force_primary_cpu_fallback_for_tests, snapshot_cpu_fallback_flags_for_tests,
};
pub(crate) use self::lifecycle::{infer, prewarm, should_retry_with_cpu_fallback_for_tests};
pub(crate) use self::loader::{
    require_quantized_snapshot_with_for_tests,
    resolve_model_kind_for_model_root_label_with_for_tests,
    resolve_model_kind_label_with_for_tests, snapshot_qoffset_alignment_with_for_tests,
};
pub(in crate::llm::vision::deepseek::native) use self::registry::{
    EngineRegistryEntryState, EngineSlot,
    clear_registry_for_tests as clear_engine_registry_for_tests,
    seed_failure_for_tests as seed_engine_failure_for_tests,
    snapshot_registry_for_tests as snapshot_engine_registry_for_tests,
};

pub(in crate::llm::vision::deepseek::native) fn store_markdown_in_cache_for_tests(
    cache_key: &str,
    markdown: &str,
) {
    cache_io::store_markdown_in_cache_for_tests(cache_key, markdown);
}

pub(in crate::llm::vision::deepseek::native) fn cache_layer_labels_for_tests()
-> (&'static str, &'static str) {
    cache_io::cache_layer_labels_for_tests()
}
