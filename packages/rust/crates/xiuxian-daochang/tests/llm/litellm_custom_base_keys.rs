//! Regression tests for transport-specific API key selection in custom-base fallback.

use xiuxian_daochang::test_support::{
    CustomBaseFallbackTransport, resolve_custom_base_transport_api_key_from_values,
};

#[test]
fn openai_transport_prefers_openai_key_over_others() {
    let key = resolve_custom_base_transport_api_key_from_values(
        CustomBaseFallbackTransport::OpenAi,
        None,
        Some("configured-key"),
        Some("openai-key"),
        Some("minimax-key"),
        Some("anthropic-key"),
    );
    assert_eq!(key.as_deref(), Some("openai-key"));
}

#[test]
fn minimax_transport_prefers_minimax_key_over_others() {
    let key = resolve_custom_base_transport_api_key_from_values(
        CustomBaseFallbackTransport::Minimax,
        None,
        Some("configured-key"),
        Some("openai-key"),
        Some("minimax-key"),
        Some("anthropic-key"),
    );
    assert_eq!(key.as_deref(), Some("minimax-key"));
}

#[test]
fn anthropic_bypass_prefers_configured_then_anthropic() {
    let configured = resolve_custom_base_transport_api_key_from_values(
        CustomBaseFallbackTransport::AnthropicMessagesBypass,
        None,
        Some("configured-key"),
        Some("openai-key"),
        Some("minimax-key"),
        Some("anthropic-key"),
    );
    assert_eq!(configured.as_deref(), Some("configured-key"));

    let anthropic = resolve_custom_base_transport_api_key_from_values(
        CustomBaseFallbackTransport::AnthropicMessagesBypass,
        None,
        None,
        Some("openai-key"),
        Some("minimax-key"),
        Some("anthropic-key"),
    );
    assert_eq!(anthropic.as_deref(), Some("anthropic-key"));
}

#[test]
fn explicit_key_always_wins_for_all_transports() {
    for transport in [
        CustomBaseFallbackTransport::OpenAi,
        CustomBaseFallbackTransport::Minimax,
        CustomBaseFallbackTransport::AnthropicMessagesBypass,
    ] {
        let key = resolve_custom_base_transport_api_key_from_values(
            transport,
            Some("explicit-key"),
            Some("configured-key"),
            Some("openai-key"),
            Some("minimax-key"),
            Some("anthropic-key"),
        );
        assert_eq!(key.as_deref(), Some("explicit-key"));
    }
}

#[test]
fn empty_values_are_treated_as_absent() {
    let key = resolve_custom_base_transport_api_key_from_values(
        CustomBaseFallbackTransport::OpenAi,
        Some("  "),
        Some(" "),
        Some("openai-key"),
        Some(" "),
        Some(" "),
    );
    assert_eq!(key.as_deref(), Some("openai-key"));
}
