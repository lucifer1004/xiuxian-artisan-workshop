//! Tests for OpenAI-compatible runtime profile resolution.

use std::collections::HashMap;
use xiuxian_llm::llm::{
    LlmProviderProfileInput, LlmRuntimeDefaults, LlmRuntimeProfileEnv, LlmRuntimeProfileInput,
    OpenAIWireApi, resolve_openai_runtime_profile,
};

#[test]
fn runtime_profile_resolves_default_provider_and_responses_wire() {
    let mut providers = HashMap::new();
    providers.insert(
        "crs".to_string(),
        LlmProviderProfileInput {
            model: Some("gpt-5-codex".to_string()),
            base_url: Some("https://openai-compatible.example.com/v1".to_string()),
            api_key: Some("CRS_OAI_KEY".to_string()),
            api_key_env: None,
            wire_api: Some("responses".to_string()),
        },
    );
    let profile = LlmRuntimeProfileInput {
        model: None,
        default_model: None,
        base_url: None,
        api_key_env: None,
        api_key: None,
        wire_api: None,
        default_provider: Some("crs".to_string()),
        providers,
    };
    let env = LlmRuntimeProfileEnv {
        provider_override: None,
        model_override: None,
        base_url_override: None,
        api_key_override: None,
        wire_api_override: None,
        env_vars: vec![
            ("OPENAI_API_KEY".to_string(), String::new()),
            ("CRS_OAI_KEY".to_string(), "crs-secret".to_string()),
        ],
    };
    let defaults = LlmRuntimeDefaults {
        provider: "openai".to_string(),
        model: "fallback-model".to_string(),
        base_url: "http://localhost:3002/v1".to_string(),
        api_key_env: "OPENAI_API_KEY".to_string(),
        wire_api: OpenAIWireApi::ChatCompletions,
    };

    let resolved = resolve_openai_runtime_profile(&profile, &env, &defaults)
        .unwrap_or_else(|err| panic!("runtime profile resolution should succeed: {err}"));

    assert_eq!(resolved.provider_name, "crs");
    assert_eq!(resolved.model, "gpt-5-codex");
    assert_eq!(
        resolved.base_url,
        "https://openai-compatible.example.com/v1"
    );
    assert_eq!(resolved.api_key_env, "CRS_OAI_KEY");
    assert_eq!(resolved.api_key, "crs-secret");
    assert_eq!(resolved.wire_api, OpenAIWireApi::Responses);
}

#[test]
fn runtime_profile_missing_api_key_env_fails() {
    let mut providers = HashMap::new();
    providers.insert(
        "openai".to_string(),
        LlmProviderProfileInput {
            model: Some("gpt-5-codex".to_string()),
            base_url: Some("https://openai-compatible.example.com/v1".to_string()),
            api_key: Some("OPENAI_API_KEY".to_string()),
            api_key_env: None,
            wire_api: Some("responses".to_string()),
        },
    );
    let profile = LlmRuntimeProfileInput {
        model: None,
        default_model: None,
        base_url: None,
        api_key_env: None,
        api_key: None,
        wire_api: None,
        default_provider: Some("openai".to_string()),
        providers,
    };
    let env = LlmRuntimeProfileEnv {
        provider_override: None,
        model_override: None,
        base_url_override: None,
        api_key_override: None,
        wire_api_override: None,
        env_vars: vec![("OPENAI_API_KEY".to_string(), String::new())],
    };
    let defaults = LlmRuntimeDefaults::default();

    let err = match resolve_openai_runtime_profile(&profile, &env, &defaults) {
        Ok(profile) => panic!("expected missing API key error, got: {profile:?}"),
        Err(err) => err,
    };
    let text = err.to_string();
    assert!(
        text.contains("missing LLM API key"),
        "unexpected error: {text}"
    );
}

#[test]
fn runtime_profile_wire_override_takes_precedence() {
    let mut providers = HashMap::new();
    providers.insert(
        "openai".to_string(),
        LlmProviderProfileInput {
            model: Some("gpt-5-codex".to_string()),
            base_url: Some("https://openai-compatible.example.com/v1".to_string()),
            api_key: Some("OPENAI_API_KEY".to_string()),
            api_key_env: None,
            wire_api: Some("responses".to_string()),
        },
    );
    let profile = LlmRuntimeProfileInput {
        model: None,
        default_model: None,
        base_url: None,
        api_key_env: None,
        api_key: None,
        wire_api: None,
        default_provider: Some("openai".to_string()),
        providers,
    };
    let env = LlmRuntimeProfileEnv {
        provider_override: None,
        model_override: None,
        base_url_override: None,
        api_key_override: None,
        wire_api_override: Some("chat_completions".to_string()),
        env_vars: vec![("OPENAI_API_KEY".to_string(), "test-openai-key".to_string())],
    };

    let resolved = resolve_openai_runtime_profile(&profile, &env, &LlmRuntimeDefaults::default())
        .unwrap_or_else(|err| panic!("runtime profile resolution should succeed: {err}"));
    assert_eq!(resolved.wire_api, OpenAIWireApi::ChatCompletions);
}
