# xiuxian-daochang

Minimal Rust agent loop (Phase B): one user turn with LLM and optional external tool integrations.

## Features

- **Config** (`AgentConfig`): inference API URL, model, API key (env or field), external-tool server list (`tool_servers`), `max_tool_rounds`, optional `window_max_turns`.
- **Session**: in-memory `SessionStore` per `session_id`; or when `window_max_turns` is set, **omni-window** (ring buffer) for bounded history and scalable context (1k–10k turns).
- **LLM** (`LlmClient`): OpenAI-compatible chat completions with optional tool definitions and `tool_calls` parsing.
- **Agent** (`Agent`): `run_turn(session_id, user_message)` — builds messages, optionally fetches external tools, calls LLM, handles tool calls, repeats until no tool_calls or `max_tool_rounds`.

## Usage

```rust
use xiuxian_daochang::{Agent, AgentConfig, ToolServerEntry};

let config = AgentConfig {
    inference_url: "https://api.openai.com/v1/chat/completions".to_string(),
    model: "gpt-4o-mini".to_string(),
    api_key: None, // uses OPENAI_API_KEY from env
    tool_servers: vec![ToolServerEntry {
        name: "local".to_string(),
        url: Some("http://127.0.0.1:3002/sse".to_string()),
        command: None,
        args: None,
    }],
    max_tool_rounds: 10,
};

let agent = Agent::from_config(config).await?;
let reply = agent.run_turn("my-session", "What's the weather?").await?;
```

## Reusing LiteLLM (no extra bridge)

The agent is an **OpenAI-compatible HTTP client**. To reuse [LiteLLM](https://docs.litellm.ai/) (100+ providers: OpenAI, Anthropic, Ollama, etc.), point `inference_url` at the LiteLLM proxy. No separate bridge process or SDK:

1. Start LiteLLM: `litellm --port 4000` (or set `LITELLM_PROXY_URL`).
2. Set `inference_url` to `http://127.0.0.1:4000/v1/chat/completions` (or use `AgentConfig::litellm("gpt-4o-mini")` which reads `LITELLM_PROXY_URL` and `OMNI_AGENT_MODEL`).
3. Use any model string LiteLLM supports: `gpt-4o`, `claude-3-5-sonnet`, `ollama/llama2`, etc. API keys are usually set in LiteLLM’s environment.

```rust
// Prefer LiteLLM so one endpoint can route to OpenAI, Anthropic, Ollama, etc.
let config = AgentConfig::litellm("gpt-4o-mini");
let agent = Agent::from_config(config).await?;
```

## Example

```bash
export OPENAI_API_KEY=sk-...
# Optional: use LiteLLM proxy (then set LITELLM_PROXY_URL or use default :4000)

cargo run -p xiuxian-daochang --example one_turn -- "Say hello in one sentence."
```

## Tests

- Unit: `cargo test -p xiuxian-daochang --test config_and_session`
- Integration (real LLM + optional external tools): `cargo test -p xiuxian-daochang --test agent_integration -- --ignored`

## Plan

See [docs/how-to/run-rust-agent.md](../../../docs/how-to/run-rust-agent.md) for the verification checklist. Phase B + C done (agent loop + gateway).
