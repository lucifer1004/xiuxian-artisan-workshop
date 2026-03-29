---
type: knowledge
metadata:
  title: "Python Packages"
---

# Python Packages

## Position

`packages/python` is a thin consumer layer around Rust-owned contracts.

Rust owns:

- the execution kernel
- routing
- memory
- indexing
- knowledge storage
- workflow/runtime orchestration
- transport/server ownership

Python retains only:

1. Arrow Flight client access
2. Arrow IPC fallback helpers
3. thin config/schema/logging helpers
4. thin consumer-side RAG enhancement helpers
5. focused adapter and contract tests

Retained runtime contract prefixes:

- `xiuxian.runtime.*`
- `xiuxian.router.*`
- `xiuxian.discover.*`
- `xiuxian_wendao.link_graph.*`

## Retained Package Set

```text
packages/python/
  xiuxian-wendao-py/   Arrow Flight transport client
  foundation/          thin config/schema/logging/RAG helpers
  core/                minimal retained helper surface
  test-kit/            focused retained fixtures/tests
```

The root workspace is explicitly limited to those four packages.
Retained Python code now ships under direct top-level packages:

- `xiuxian_core`
- `xiuxian_foundation`
- `xiuxian_rag`
- `xiuxian_test_kit`
- `xiuxian_tracer`
- `xiuxian_wendao_py`

## Removed Surface

The historical Python runtime-center architecture is gone. This includes the
former `agent` package, `mcp-server`, `xiuxian_core.skills`, and the old
Python-local router, memory, workflow, knowledge-host, bindings, MCP, watcher,
scanner, hot-reload, and skill-runner stacks.

The old `src/omni/...` namespace layout is gone as well.

## Rules

1. Python is not a peer runtime center.
2. Arrow Flight is the default Python integration path.
3. Arrow IPC is the sanctioned fallback.
4. New Python code must stay transport-consumer-only or helper-only.
5. If Rust already owns a responsibility, Python must not recreate it behind a
   compatibility label.
6. Delete stale local-runtime surfaces rather than preserving them as legacy
   architecture.

## Documentation Notes

`P0_surface_inventory.md`, `P1_retirement_matrix.md`, and the developer guides
now describe only the retained Python scope. Historical references to deleted
Python runtime surfaces should be treated as archive material only.
