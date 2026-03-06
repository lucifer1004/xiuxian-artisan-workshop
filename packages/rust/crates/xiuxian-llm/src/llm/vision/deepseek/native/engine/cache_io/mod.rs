mod layer;
mod read;
mod write;

pub(super) use self::layer::CacheLayer;
pub(super) use self::read::read_cache_entry;
pub(in crate::llm::vision::deepseek::native) use self::write::store_markdown_in_cache_for_tests;
pub(in crate::llm::vision::deepseek::native::engine) use self::write::{
    non_empty_markdown, store_markdown_in_cache,
};

pub(in crate::llm::vision::deepseek::native) fn cache_layer_labels_for_tests()
-> (&'static str, &'static str) {
    (CacheLayer::Local.as_str(), CacheLayer::Valkey.as_str())
}
