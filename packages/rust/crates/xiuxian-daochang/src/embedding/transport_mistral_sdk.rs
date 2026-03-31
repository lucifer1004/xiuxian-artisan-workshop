pub(super) async fn embed_mistral_sdk(
    _texts: &[String],
    _model: Option<&str>,
    _hf_cache_path: Option<&str>,
    _hf_revision: Option<&str>,
    _max_num_seqs: Option<usize>,
) -> Option<Vec<Vec<f32>>> {
    tracing::warn!(
        "mistral_sdk embedding backend is unavailable in this build; enable the local xiuxian-llm mistral runtime wiring before selecting it"
    );
    None
}
