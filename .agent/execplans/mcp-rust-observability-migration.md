# MCP Rust Observability Migration

## Purpose / Big Picture
Migrate the strongest operational traits from the Python MCP implementation into the Rust MCP path that powers `xiuxian-llm` and `xiuxian-daochang`: explicit connection diagnostics, structured server-notification logging, and prompt cache invalidation when the server broadcasts tool-list changes. Keep protocol mechanics in `xiuxian-mcp`, while letting `xiuxian-llm::mcp` consume the new client signals.

## Progress
- [x] Audit Python MCP orchestration and logging behavior.
- [x] Locate the upstream Rust MCP dependency via `cargo metadata` and identify notification hooks in `rmcp`.
- [x] Add client-side notification state + structured logs to `xiuxian-mcp`.
- [x] Wire `tools/list_changed` invalidation into `xiuxian-llm::mcp` pool caching.
- [x] Add focused Rust regression tests.
- [x] Run targeted validation (`nextest`, `clippy`).

## Surprises & Discoveries
- The production MCP pool is not in `xiuxian-mcp`; it lives in `xiuxian-llm::mcp` and re-exports through `xiuxian-daochang`.
- `rmcp` 0.16 already exposes exactly the hooks we need through `ClientHandler`, especially `on_tool_list_changed` and `on_logging_message`.
- Rust already had strong pool-level wait/connect diagnostics, but the bottom client layer was effectively notification-blind.
- Re-running Rust validation in the shared workspace kept blocking on existing cargo locks; isolating with `CARGO_TARGET_DIR=.cache/codex-mcp-audit-target` avoided interference and exposed the real compile cost of Metal-backed `xiuxian-llm`.

## Decision Log
- Keep protocol-facing notification handling in `xiuxian-mcp`.
- Keep pool/cache policy in `xiuxian-llm::mcp`; do not move it into `xiuxian-daochang`.
- Prioritize `tools/list_changed` correctness and server log visibility before any broader MCP server rewrite.
- Use a process-wide monotonic epoch for tool-list-change notifications so reconnects cannot regress cache invalidation ordering.

## Outcomes & Retrospective
- `xiuxian-mcp` now uses an `rmcp::ClientHandler` implementation instead of a notification-blind raw init payload, and it surfaces notification counters plus structured tracing.
- `xiuxian-llm::mcp::McpClientPool` now invalidates cached `tools/list` data when any pooled client receives `notifications/tools/list_changed`.
- Targeted regression tests now cover both notification reception and pool cache invalidation.

## Context and Orientation
- Python reference implementation:
  - `packages/python/mcp-server/src/omni/mcp/server.py`
  - `packages/python/mcp-server/src/omni/mcp/gateway.py`
  - `packages/python/mcp-server/src/omni/mcp/transport/stdio.py`
  - `packages/python/mcp-server/src/omni/mcp/transport/sse.py`
- Rust protocol crate:
  - `packages/rust/crates/xiuxian-mcp/src/client.rs`
- Rust pool/runtime path:
  - `packages/rust/crates/xiuxian-llm/src/mcp/pool.rs`
  - `packages/rust/crates/xiuxian-llm/src/mcp/pool/list_ops.rs`
  - `packages/rust/crates/xiuxian-llm/src/mcp/pool/lifecycle.rs`
- Upstream dependency:
  - `/Users/guangtao/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rmcp-0.16.0/src/handler/client.rs`
  - `/Users/guangtao/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rmcp-0.16.0/src/service/client.rs`
  - `/Users/guangtao/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/rmcp-0.16.0/src/model.rs`

## Plan of Work
1. Introduce an internal `ClientHandler` implementation in `xiuxian-mcp` that records server notifications and logs them with connection context.
2. Extend `OmniMcpClient` with read-only diagnostics accessors for notification counters / epochs.
3. Update `xiuxian-llm::mcp::McpClientPool` to invalidate the `tools/list` cache when any pooled client reports a newer tool-list-change epoch.
4. Add integration coverage using the existing mock `rmcp` server.

## Concrete Steps
1. Refactor `packages/rust/crates/xiuxian-mcp/src/client.rs` to replace raw `InitializeRequestParams` service boot with a custom `ClientHandler` wrapper.
2. Add public `McpClientNotificationStats` re-exports to `xiuxian-mcp` and `xiuxian-llm::mcp`.
3. Update `packages/rust/crates/xiuxian-llm/src/mcp/pool.rs` and `packages/rust/crates/xiuxian-llm/src/mcp/pool/list_ops.rs` to consult client notification epochs before serving cached `tools/list` results.
4. Add regression tests under `packages/rust/crates/xiuxian-mcp/tests/notifications.rs` and extend `packages/rust/crates/xiuxian-llm/tests/mcp_pool_runtime.rs`.

## Validation and Acceptance
- `cargo nextest run -p xiuxian-mcp --status-level all` with `CARGO_TARGET_DIR=.cache/codex-mcp-audit-target`
- `cargo clippy -p xiuxian-mcp -- -W clippy::too_many_lines` with `CARGO_TARGET_DIR=.cache/codex-mcp-audit-target`
- `cargo nextest run -p xiuxian-llm -E 'test(mcp_pool_list_tools_cache_serves_second_request_from_cache) or test(mcp_pool_invalidates_cached_tools_after_server_tool_list_changed) or test(mcp_pool_discover_cache_stats_absent_when_not_configured) or test(mcp_facade_reexports_client_surface)' --status-level all` with `CARGO_TARGET_DIR=.cache/codex-mcp-audit-target`
- `cargo clippy -p xiuxian-llm -- -W clippy::too_many_lines` with `CARGO_TARGET_DIR=.cache/codex-mcp-audit-target`

## Idempotence and Recovery
- Client notification state is additive and safe to re-read.
- Pool cache invalidation only clears local cached tool metadata; a failed refresh naturally falls back to the existing uncached request path.
- If notification wiring causes regressions, reverting the `ClientHandler` wrapper and the epoch check is isolated to MCP files.

## Artifacts and Notes
- This work intentionally avoids public provider URLs or customer-specific endpoints.
- The Python implementation remains the behavioral reference for broadcast/logging semantics.

## Interfaces and Dependencies
- Depends on `rmcp` client hooks (`ClientHandler`, `on_tool_list_changed`, `on_logging_message`).
- `xiuxian-llm` depends on `xiuxian-mcp` for the client surface.
- `xiuxian-daochang` remains a thin integration layer.

## Change Log
- 2026-03-06: Created plan after auditing Python MCP, `xiuxian-mcp`, `xiuxian-llm::mcp`, and upstream `rmcp`.
- 2026-03-06: Implemented `ClientHandler`-based observability in `xiuxian-mcp` and added notification counters.
- 2026-03-06: Added `tools/list_changed` cache invalidation in `xiuxian-llm::mcp` and covered it with runtime tests.
- 2026-03-06: Completed targeted `nextest` and `clippy` validation using an isolated cargo target directory.
