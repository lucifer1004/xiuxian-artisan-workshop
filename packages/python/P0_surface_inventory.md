---
type: knowledge
metadata:
  title: "P0 Python Surface Inventory"
---

# P0 Python Surface Inventory

## Current State

The historical inventory phase is complete. The Python tree has already been
collapsed to the retained post-deletion surface:

1. `xiuxian-wendao-py`
   Arrow Flight / Arrow IPC transport client and thin compatibility helpers.
2. `foundation`
   Thin config, schema, logging, and RAG-enhancement helpers.
3. `core`
   Minimal retained compatibility/helpers around kernel-adjacent utilities.

The old runtime-center packages are no longer present:

- `packages/python/agent/` has been physically deleted.
- the removed standalone protocol server package has been physically deleted.
- `xiuxian_core.skills` has been physically deleted.
- Python-local router, workflow, memory, knowledge-host, and legacy protocol-host package
  surfaces have been physically deleted.

## Retained Public Surface

### `xiuxian_wendao_py`

Retained role:

- Arrow Flight client access
- Arrow IPC fallback helpers
- thin compatibility/config/runtime utilities that still serve transport
  consumers

### `xiuxian_foundation`

Retained role:

- PRJ path/config helpers
- schema/resource lookup
- logging
- thin vector/search and RAG-enhancement helpers that do not own storage or
  execution

### `xiuxian_core`

Retained role:

- minimal kernel/context/security helpers that remain useful without recreating
  a Python runtime center

## Removed Surface

These surfaces no longer belong to the Python architecture and have already
been removed:

1. `omni.agent`
2. the removed protocol-host package surface
3. `xiuxian_core.skills`
4. Python-local router/indexer stacks
5. Python-local knowledge ingestion/indexing stacks
6. Python-local memory services and workflow stacks
7. Python-local bindings wrappers around `xiuxian_core_rs` / `omni_core_rs`
8. Python-local skill runner, watcher, hot-reload, and scanner stacks

## Architectural Rule

Python is not an application center. Python is a thin consumer layer around
Rust-owned contracts. If a new Python module attempts to own execution, memory,
routing, indexing, or transport semantics already owned by Rust/Wendao, it is
out of bounds and should be deleted rather than preserved behind a shim.
