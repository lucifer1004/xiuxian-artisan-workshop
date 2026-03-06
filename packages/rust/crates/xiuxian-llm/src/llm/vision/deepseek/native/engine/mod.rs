mod batch_lane;
mod cache_io;
mod coalescer;
mod core;
mod image_decode;
mod lifecycle;
mod loader;
mod retry;
mod telemetry;

pub(crate) use self::lifecycle::{infer, prewarm, should_retry_with_cpu_fallback_for_tests};
pub(crate) use self::loader::{
    require_quantized_snapshot_with_for_tests,
    resolve_model_kind_for_model_root_label_with_for_tests,
    resolve_model_kind_label_with_for_tests, snapshot_qoffset_alignment_with_for_tests,
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
