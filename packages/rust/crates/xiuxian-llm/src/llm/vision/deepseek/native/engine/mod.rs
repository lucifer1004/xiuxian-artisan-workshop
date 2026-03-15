mod batch_lane;
mod cache_io;
mod coalescer;
mod core;
mod dsq_repair;
mod image_decode;
mod lifecycle;
mod loader;
mod retry;
mod telemetry;

pub use self::dsq_repair::{DsqRepairResult, repair_dsq_if_needed};
pub(crate) use self::lifecycle::{infer, prewarm, should_retry_with_cpu_fallback_for_tests};
pub(crate) use self::loader::{
    require_quantized_snapshot_with_for_tests,
    resolve_model_kind_for_model_root_label_with_for_tests,
    resolve_model_kind_label_with_for_tests, snapshot_qoffset_alignment_with_for_tests,
};

pub(in crate::llm::vision::deepseek) fn store_markdown_in_cache_for_tests(
    cache_key: &str,
    markdown: &str,
) {
    cache_io::store_markdown_in_cache_for_tests(cache_key, markdown);
}
