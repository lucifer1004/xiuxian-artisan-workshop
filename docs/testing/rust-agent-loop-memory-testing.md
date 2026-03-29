---
type: knowledge
title: "Rust Agent: Loop, ReAct, and Memory Testing"
category: "testing"
tags:
  - testing
  - rust
saliency_base: 6.5
decay_rate: 0.04
metadata:
  title: "Rust Agent: Loop, ReAct, and Memory Testing"
---

# Rust Agent: Loop, ReAct, and Memory Testing

> How the xiuxian-daochang loop aligns with Nanobot/ZeroClaw and how we test loop, ReAct (tool roundtrip), and xiuxian-memory-engine in one place.

## 1. Loop and ReAct vs Nanobot / ZeroClaw

### Reference behaviour (Nanobot, ZeroClaw)

- **Nanobot**: One `AgentLoop`; each message → session (by `channel:chat_id`) → context (history + memory window) → **LLM → tools → repeat** until done → publish. External tools live in the same effective registry as built-in tools.
- **ZeroClaw**: Rust, one agent loop; gateway/daemon/service are modes of the same runtime; trait-based Provider/Channel/Memory/Tool.

### Our loop (xiuxian-daochang)

One **ReAct cycle** per user turn is implemented in `Agent::run_turn`:

1. **Optional recall**: If memory is enabled, run `two_phase_recall(intent)` and inject "Relevant past experiences" as first system message.
2. **Build messages**: Session history + user message (and recall context when present).
3. **LLM**: Call OpenAI-compatible `/v1/chat/completions` with optional `tools`.
4. **Tool round**: If the LLM returns `tool_calls`, call each via the external tool runtime, append results to messages, then go back to step 3 (up to `max_tool_rounds`).
5. **Final reply**: When the LLM returns no `tool_calls`, append the turn to session and, if memory is enabled, **store_episode** (intent + experience + outcome).

So: **one `run_turn` = one Nanobot-style “process message”** (session + optional recall → LLM ↔ tools loop → store). Gateway/stdio only differ by transport; the loop is the same.

## 2. Test layers

| Layer           | What                                                                                      | Where                                                               |
| --------------- | ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| **Unit**        | Config, session, qualify/parse, agent `from_config` with memory                           | `config_and_session`, `multiple_mcp`, `config_mcp`, `gateway_stdio` |
| **Integration** | One turn with real LLM; one turn with LLM + external tools (ReAct); two turns with memory | `agent_integration`                                                 |

### What each test covers

| Test                                      | Loop (LLM) | ReAct (tool roundtrip)                    | Memory (recall + store)           |
| ----------------------------------------- | ---------- | ----------------------------------------- | --------------------------------- |
| `config_and_session`                      | —          | —                                         | `from_config` with `MemoryConfig` |
| `agent_from_config_with_memory_succeeds`  | —          | —                                         | Agent builds with memory          |
| `test_agent_one_turn_with_llm_and_mcp`    | ✓          | ✓ (if model uses a tool)                  | —                                 |
| `test_agent_one_turn_litellm_project_mcp` | ✓          | ✓ (if an external tool + model uses tool) | —                                 |
| **`test_agent_react_flow_tool_used`**     | ✓          | **✓ (required)**                          | —                                 |
| `test_agent_two_turns_memory_stored`      | ✓          | optional                                  | ✓ (episode count ≥ 2)             |

- **Loop**: Any integration test that calls `run_turn` and gets a non-empty reply exercises the loop (LLM call, optional tool rounds, final reply).
- **ReAct (same as Nanobot/ZeroClaw)**: `test_agent_react_flow_tool_used` **requires** the model to use a tool: it sends a prompt that asks the model to call the echo (demo) tool with a fixed string; the test asserts the final reply contains that string, proving the path **LLM → tool_calls → tool execution → LLM → final reply** ran.
- **Memory**: Unit test ensures the agent builds with memory; `test_agent_two_turns_memory_stored` runs two turns and asserts `test_episode_count() >= 2`.

## 3. Running the tests

### Unit tests (no network, no API key)

```bash
cargo test -p xiuxian-daochang --test config_and_session
cargo test -p xiuxian-daochang --test multiple_mcp
cargo test -p xiuxian-daochang --test config_mcp
cargo test -p xiuxian-daochang --test gateway_stdio
```

### Integration tests (real LLM and/or external tools; run with `--ignored`)

Require env:

- `LITELLM_PROXY_URL` or `XIUXIAN_DAOCHANG_INFERENCE_URL` (inference endpoint)
- `XIUXIAN_DAOCHANG_MODEL` (model name)
- `OPENAI_API_KEY` or `XIUXIAN_DAOCHANG_INFERENCE_API_KEY` (unless inference is local)

```bash
# One turn (loop; ReAct if a tool-enabled model uses a tool)
cargo test -p xiuxian-daochang --test agent_integration test_agent_one_turn_litellm_project_mcp -- --ignored

# ReAct flow: model must use a tool (loop + tool roundtrip, like Nanobot/ZeroClaw)
cargo test -p xiuxian-daochang --test agent_integration test_agent_react_flow_tool_used -- --ignored

# Two turns with memory (loop + memory store)
cargo test -p xiuxian-daochang --test agent_integration test_agent_two_turns_memory_stored -- --ignored
```

To run all integration tests:

```bash
cargo test -p xiuxian-daochang --test agent_integration -- --ignored
```

## 4. Unified “matrix” (what to run for parity)

To approximate Nanobot/ZeroClaw behaviour in one go:

1. **Unit**: All unit tests above (config, session, gateway/stdio, agent with memory from_config).
2. **Loop + ReAct**: Run **`test_agent_react_flow_tool_used`** — this explicitly requires the model to call a tool (echo) and asserts the reply contains the tool result, matching the Nanobot/ZeroClaw “model uses tool” flow.
3. **Memory**: `test_agent_two_turns_memory_stored` (two turns, then assert episode count ≥ 2).

Together these cover: **loop**, **ReAct (tool roundtrip, same as reference projects)**, and **memory (recall + store)** in a single test suite.

## 5. Session Compression Tracking (2026-02-18)

This round adds rolling session compression in `xiuxian-daochang` so long-running channel sessions
can keep recent turns while preserving older context as compact summaries.

### Implemented

- Added `SessionSummarySegment` (session summary record) and persistence path:
  - in-memory bounded store
  - Valkey/Redis backend (`xiuxian-daochang:session:summary:<session_id>`)
- Consolidation now does both:
  - drains oldest turns and stores one episode into `xiuxian-memory-engine`
  - stores one compact summary segment for prompt reuse
- Prompt context for bounded sessions now includes:
  - compacted summary segments (old history)
  - recent window turns (working memory)

### Config added

- `summary_max_segments` (default `8`)
- `summary_max_chars` (default `480`)
- `consolidation_async` (default `true`)
- `context_budget_tokens` (optional total context budget)
- `context_budget_reserve_tokens` (default `512`)

Paths and env:

- `session.summary_max_segments` and `session.summary_max_chars` in `settings.yaml`
- `session.consolidation_async`
- `session.context_budget_tokens`
- `session.context_budget_reserve_tokens`
- `XIUXIAN_DAOCHANG_SUMMARY_MAX_SEGMENTS`
- `XIUXIAN_DAOCHANG_SUMMARY_MAX_CHARS`
- `XIUXIAN_DAOCHANG_CONSOLIDATION_ASYNC`
- `XIUXIAN_DAOCHANG_CONTEXT_BUDGET_TOKENS`
- `XIUXIAN_DAOCHANG_CONTEXT_BUDGET_RESERVE_TOKENS`

### Tests added/updated

- `tests/session_summary.rs`
  - bounded summary retention + trimming
  - clear session removes both window and summary data
- `tests/agent_context_budget.rs`
  - latest non-system message retention under budget
  - reserve-token behavior
  - truncation behavior for oversized content
- `tests/config_settings.rs`
  - user-over-system merge for summary settings
- Updated explicit `AgentConfig` builders in integration/unit examples to include summary fields

### Verification

- Commands:
  - `cargo test -p xiuxian-daochang --test agent_summary --test agent_context_budget --test config_settings -q`
  - `cargo test -p xiuxian-daochang --test session_summary -q`
- Result on **2026-02-18**: pass for targeted memory/session tests

## 6. References

- [omni-run-react-gateway-design.md](../plans/omni-run-react-gateway-design.md) — Nanobot analysis, session window, gateway.
- [omni-run-roadmap-nanobot-zeroclaw.md](../plans/omni-run-roadmap-nanobot-zeroclaw.md) — Product goal and phased roadmap.
- [rust-agent-architecture-omni-vs-zeroclaw.md](../plans/rust-agent-architecture-omni-vs-zeroclaw.md) — Architecture comparison.

**Local reference implementations (researcher cache):**

- ZeroClaw source: `.cache/researcher/zeroclaw-labs/zeroclaw/`
- Other harvested/cloned repos: `.cache/researcher/`
