---
type: knowledge
metadata:
  title: "Omni Foundation"
---

# Xiuxian Foundation

Shared kernel and utilities for retained xiuxian-artisan-workshop runtime helpers.

## Overview

This package provides shared helper components for retained Python consumers.

## Core Modules

| Module                        | Purpose                        |
| ----------------------------- | ------------------------------ |
| `xiuxian_foundation.config`   | Settings, paths, logging       |
| `xiuxian_foundation.api`      | Decorators, protocols, types   |
| `xiuxian_foundation.services` | LLM, memory, embedding, vector |
| `xiuxian_foundation.runtime`  | Context, isolation, gitops     |

## Services Submodules

### Memory Module (`xiuxian_foundation.services.memory`)

Project memory storage using ADR pattern with LanceDB backend.

```
xiuxian_foundation.services.memory/
├── base.py                    # Public API exports
├── core/
│   ├── interface.py           # Abstract interfaces and data types
│   ├── project_memory.py      # ProjectMemory main class
│   └── utils.py               # Shared utilities
└── stores/
    └── lancedb.py             # LanceDB storage (single backend)
```

### Key Classes

```python
from xiuxian_foundation.services.memory import ProjectMemory

# Create memory instance (LanceDB by default)
memory = ProjectMemory()

# Add decision
memory.add_decision(
    title="Use LanceDB for Memory Storage",
    problem="File-based storage is slow",
    solution="Migrate to LanceDB",
    status="accepted",
)

# List decisions
decisions = memory.list_decisions()
```

See [Memory Module Reference](../../../../docs/reference/memory-module.md) for full documentation.

## Dependencies

- Anthropic SDK for LLM integration
- Structlog for structured logging
- LanceDB for vector storage
