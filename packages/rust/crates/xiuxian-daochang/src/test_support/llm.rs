pub use crate::llm::test_api::{
    ChatCompletionRequest, DEFAULT_ANTHROPIC_KEY_ENV, DEFAULT_MINIMAX_KEY_ENV,
    DEFAULT_OPENAI_KEY_ENV, LiteLlmProviderMode, LiteLlmWireApi, LlmBackendMode, ProviderSettings,
    ToolMessageIntegrityReport, chat_completion_request_to_value, enforce_tool_message_integrity,
    extract_api_base_from_inference_url, is_openai_like_stream_required_error, parse_backend_mode,
    parse_tools_json, resolve_provider_settings_with_env, should_use_openai_like_for_base,
};

#[cfg(feature = "agent-provider-litellm")]
pub use crate::llm::test_api::{
    CustomBaseFallbackTransport, OcrGateTimeoutRecoveryProbe,
    build_responses_payload_from_chat_completion_request, chat_message_to_litellm_message,
    deepseek_ocr_memory_guard_triggered, infer_deepseek_ocr_truth_from_image_bytes,
    parse_responses_stream_tool_names, resolve_custom_base_transport_api_key_from_values,
    resolve_deepseek_ocr_global_lock_path, resolve_deepseek_ocr_memory_limit_bytes,
    simulate_ocr_gate_panic_recovery, simulate_ocr_gate_timeout_recovery,
};
