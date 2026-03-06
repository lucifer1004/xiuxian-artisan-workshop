# Responses Tool Call Integrity

## Purpose / Big Picture
Stabilize the OpenAI-compatible `/responses` tool-call roundtrip used by `xiuxian-daochang` and `xiuxian-llm`. The immediate production symptom is `No tool call found for function call output with call_id ...` after a successful first tool-call round.

## Progress
- Investigating the live failure path from `xiuxian-daochang` into `xiuxian-llm`.
- Confirmed the current production path does not rely on `call_legacy_*` fallback IDs.
- Implemented targeted regression coverage and request-side integrity checks.
- Added `/responses` input diagnostics so live 400s now log a compact `input_summary` before and at failure time.
- Separated long-lived webhook/Discord runtime launches from `cargo run` by switching them to build-then-exec against the shared workspace `target/` directory.

## Surprises & Discoveries
- `xiuxian-daochang` currently serializes tool messages into `LiteContentPart::ToolResult`, but `xiuxian-llm` only extracts `ContentPart::Text` when building `/responses` `function_call_output`, which can silently erase tool output text.
- The live failure occurs on the second `/responses` dispatch after the first tool call and tool execution both succeed.
- `react_loop` already writes the original assistant `tool_call.id` back onto `ChatMessage.tool_call_id`; adding `tool_call_id` onto `ToolCallOutput` is therefore a contract-hardening step, not proof that this field omission was the sole root cause.
- No in-repo sample currently shows a non-standard pipe-delimited ID shape such as `call_id|fc_id`, so compatibility parsing remains deferred until a concrete gateway payload is captured.

## Decision Log
- Treat `call_legacy_*` as a dead-path concern for this runtime until proven otherwise.
- Add end-to-end tests through the real `ChatMessage -> LiteChatRequest -> /responses payload` path instead of relying only on lower-level unit tests.
- Add request-side validation for malformed `function_call_output` chains before sending to the provider.
- Preserve the originating `tool_call_id` across internal tool dispatch (`ToolCallOutput`, `NativeToolCallContext`, and degraded tool outputs) so every tool result keeps a first-class correlation handle.
- Normalize pipe-delimited call IDs (`call_id|fc_id`) down to the leading protocol ID at integrity-check and payload-emission boundaries, but defer any broader gateway-specific heuristics until a real sample appears.
- Start the architecture lift by extracting the existing integrity logic into an `llm::protocol::hygiene` module before introducing any policy trait or middleware abstraction.
- Introduce `HygienePolicy` only after the module boundary exists, and keep the first implementation behavior-identical via `OpenAiHygienePolicy`.

## Outcomes & Retrospective
- In progress.
- Webhook and Discord long-lived launchers now build once into the shared workspace `target/` directory and exec the binary directly, eliminating runtime `cargo run` lock contention without introducing a second runtime target tree.

## Context and Orientation
- Runtime entry: `packages/rust/crates/xiuxian-daochang/src/llm/client/chat.rs`
- Message conversion: `packages/rust/crates/xiuxian-daochang/src/llm/converters.rs`
- `/responses` payload builder: `packages/rust/crates/xiuxian-llm/src/llm/providers/openai_like/responses.rs`
- Test API bridge: `packages/rust/crates/xiuxian-daochang/src/llm/test_api.rs`

## Plan of Work
1. Add regression coverage for the daochang end-to-end builder path with assistant tool calls and tool outputs.
2. Fix tool-output serialization so `ToolResult` content is preserved in `function_call_output.output`.
3. Add a local tool-chain integrity validator before `/responses` requests are sent.
4. Add compact runtime diagnostics for the final `/responses` `input` sequence so live provider 400s can be correlated with the exact tool-call chain sent upstream.
5. Run targeted Rust validation for touched crates.

## Concrete Steps
1. Update `packages/rust/crates/xiuxian-daochang/tests/llm/backend.rs` with an end-to-end `/responses` payload regression.
2. Update `packages/rust/crates/xiuxian-llm/tests/llm_openai_responses_payload.rs` with a `ToolResult` serialization regression.
3. Patch `packages/rust/crates/xiuxian-llm/src/llm/providers/openai_like/responses.rs` to preserve tool output text, validate tool-call chains, and reject duplicate `function_call_output` consumption.
4. Patch `packages/rust/crates/xiuxian-llm/src/llm/providers/openai_like.rs` to log a compact `input_summary` before `/responses` sends and on client-side 4xx failures.
5. Run targeted Rust validation for the touched crates and tests.

## Validation and Acceptance
- `cargo nextest run -p xiuxian-llm --test llm_openai_responses_payload`
- `cargo nextest run -p xiuxian-daochang --test backend`
- `cargo check -p xiuxian-daochang`
- `cargo clippy -p xiuxian-llm -- -W clippy::too_many_lines`
- `cargo clippy -p xiuxian-daochang -- -W clippy::too_many_lines`

## Idempotence and Recovery
- Test additions are deterministic and can be rerun repeatedly.
- If request-side validation trips, the error should surface locally before any upstream mutation.
- If live behavior still fails after these checks pass, the remaining fault is likely provider-specific semantics rather than local call-id loss.

## Artifacts and Notes
- Live failing log reference: `.run/logs/xiuxian-daochang-webhook.log:31221`
- Live failing call id: `call_80R7yIHwiyPlDzfvFCAO3ZTC`

## Interfaces and Dependencies
- `xiuxian-daochang` depends on `xiuxian-llm` test helpers for building `/responses` payloads.
- OpenAI-compatible `/responses` transport runs through `reqwest` in `xiuxian-llm`.

## Change Log
- 2026-03-06: Created plan and scoped the first regression/fix batch.
- 2026-03-06: Switched webhook and Discord ingress runtime launchers from `cargo run` to build-then-exec against the shared workspace `target/` directory.
- 2026-03-06: Hardened internal tool dispatch to preserve `tool_call_id` on native/zhenfa/MCP outputs and in native tool invocation context.
- 2026-03-06: Added `run_turn`-level regression assertions for MCP, native zhenfa, and native zhixing tool-result `tool_call_id` preservation.
- 2026-03-06: Added pipe-delimited call-id normalization in `xiuxian-daochang` integrity/conversion layers and in `xiuxian-llm` `/responses` payload assembly/validation.
- 2026-03-06: Extracted tool-chain hygiene into `xiuxian-daochang::llm::protocol::hygiene` without changing behavior.
- 2026-03-06: Added `HygienePolicy` with a behavior-equivalent `OpenAiHygienePolicy`, and wired `LlmClient::chat` through that default policy.

## Latest Validation
- `cargo check -p xiuxian-llm`
- `cargo nextest run -p xiuxian-llm --test llm_openai_responses_transport -E 'test(execute_openai_responses_request_rejects_duplicate_tool_outputs_before_send) | test(execute_openai_responses_request_rejects_orphan_tool_outputs_before_send)'`
- `cargo nextest run -p xiuxian-llm --test llm_openai_responses_payload`
- `cargo clippy -p xiuxian-llm -- -W clippy::too_many_lines`
- `cargo nextest run -p xiuxian-daochang --test llm tool_message_integrity_normalizes_pipe_delimited_ids`
- `cargo nextest run -p xiuxian-llm --test llm_openai_responses_payload responses_payload_normalizes_pipe_delimited_tool_call_ids`
- `cargo check -p xiuxian-daochang -p xiuxian-llm`
- `cargo clippy -p xiuxian-llm -- -W clippy::too_many_lines`
- `cargo clippy -p xiuxian-daochang -- -W clippy::too_many_lines`
- `cargo nextest run -p xiuxian-daochang --test llm`
- `cargo check -p xiuxian-daochang`
- `cargo nextest run -p xiuxian-daochang --test llm`
- `cargo clippy -p xiuxian-daochang -- -W clippy::too_many_lines`

## Remaining Work
- Restart the webhook runtime and reproduce one tool-call turn to capture the new `xiuxian.llm.providers.openai_like.responses.dispatch` / `...failed` log entries.
- Compare the logged `input_summary` against the upstream 400. If the summary still shows a valid local chain, the remaining bug is provider-side semantics around replayed chains rather than local call-id loss.
- Evaluate Ghost Call Synthesis only if a real proactive-native path starts writing `role=tool` messages without a preceding assistant `tool_calls` block; current audited runtime paths do not do this.
