#![cfg(feature = "llm")]

use futures::StreamExt;
use xiuxian_llm::llm::{ChatRequest, LlmClient, OpenAICompatibleClient, OpenAIWireApi};
use xiuxian_qianji::runtime_config::resolve_qianji_runtime_llm_config;

fn live_runtime_default_enabled() -> bool {
    std::env::var("XIUXIAN_QIANJI_LIVE_LLM")
        .ok()
        .map(|raw| raw.trim().to_ascii_lowercase())
        .is_some_and(|raw| matches!(raw.as_str(), "1" | "true" | "yes" | "on"))
}

fn build_runtime_default_client() -> (OpenAICompatibleClient, String) {
    let runtime = resolve_qianji_runtime_llm_config()
        .unwrap_or_else(|error| panic!("runtime default llm config should resolve: {error}"));
    let model = runtime.model.clone();
    let client = OpenAICompatibleClient {
        api_key: runtime.api_key,
        base_url: runtime.base_url,
        wire_api: OpenAIWireApi::parse(Some(runtime.wire_api.as_str())),
        http: reqwest::Client::new(),
    };
    (client, model)
}

#[tokio::test]
async fn runtime_default_live_llm_chat_round_trip() {
    if !live_runtime_default_enabled() {
        return;
    }

    let (client, model) = build_runtime_default_client();
    let request = ChatRequest::new(model)
        .add_system_message("Reply with XML only and include a <score> tag.")
        .add_user_message("Return <score>0.95</score> and one short rationale sentence.")
        .with_temperature(0.1)
        .with_max_tokens(128);

    let response = client
        .chat(request)
        .await
        .unwrap_or_else(|error| panic!("runtime default live chat should succeed: {error}"));

    assert!(
        response.contains("<score>") || !response.trim().is_empty(),
        "runtime default live chat returned unexpected output: {response}"
    );
}

#[tokio::test]
async fn runtime_default_live_llm_stream_round_trip() {
    if !live_runtime_default_enabled() {
        return;
    }

    let (client, model) = build_runtime_default_client();
    let request = ChatRequest::new(model)
        .add_system_message("Reply with XML only and include a <score> tag.")
        .add_user_message("Return <score>0.95</score> and one short rationale sentence.")
        .with_temperature(0.1)
        .with_max_tokens(128);

    let mut stream = client
        .chat_stream(request)
        .await
        .unwrap_or_else(|error| panic!("runtime default live stream should start: {error}"));

    let mut content = String::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk
            .unwrap_or_else(|error| panic!("runtime default live stream chunk failed: {error}"));
        content.push_str(&chunk);
        if content.contains("<score>") {
            break;
        }
    }

    assert!(
        content.contains("<score>") || !content.trim().is_empty(),
        "runtime default live stream returned unexpected output: {content}"
    );
}
