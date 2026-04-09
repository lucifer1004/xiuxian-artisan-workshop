//! Test coverage for xiuxian-daochang behavior.

//! Integration test: Agent with mock or real external tools (when available).
//! Run with real LLM/tools: set `OPENAI_API_KEY` (or `ANTHROPIC_API_KEY`),
//! optionally start a tool server on port 3002,
//! then `cargo test -p xiuxian-daochang --test agent_integration -- --ignored`.

use xiuxian_daochang::{Agent, AgentConfig, ToolServerEntry};

fn default_config() -> AgentConfig {
    AgentConfig {
        inference_url: std::env::var("OMNI_AGENT_INFERENCE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1/chat/completions".to_string()),
        model: std::env::var("OMNI_AGENT_MODEL").unwrap_or_else(|_| "gpt-4o-mini".to_string()),
        api_key: None,
        tool_servers: vec![ToolServerEntry {
            name: "local".to_string(),
            url: Some(
                std::env::var("OMNI_AGENT_TOOL_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:3002/sse".to_string()),
            ),
            command: None,
            args: None,
        }],
        max_tool_rounds: 5,
        ..AgentConfig::default()
    }
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY and optional tool server on 3002; run with --ignored"]
async fn test_agent_one_turn_with_llm_and_tools() {
    let config = default_config();
    if config.resolve_api_key().is_none() {
        eprintln!("skip: no API key");
        return;
    }
    let agent = match Agent::from_config(config).await {
        Ok(agent) => agent,
        Err(error) => panic!("agent from_config: {error}"),
    };
    let out = match agent
        .run_turn("test-session", "Say hello in one short sentence.")
        .await
    {
        Ok(out) => out,
        Err(error) => panic!("run_turn: {error}"),
    };
    assert!(!out.is_empty());
}
