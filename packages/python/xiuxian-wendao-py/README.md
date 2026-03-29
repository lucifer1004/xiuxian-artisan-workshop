---
type: knowledge
metadata:
  title: "xiuxian-wendao-py"
---

# xiuxian-wendao-py

`xiuxian-wendao-py` is the transport-first Python package for the
`xiuxian-wendao` Rust runtime.

It exists to give Python consumers a stable `xiuxian_*` entrypoint while the
repository retires the old `omni.*` package family.

The default architecture is:

1. Rust owns execution and state.
2. Python prefers Arrow Flight transport.
3. Python falls back to Arrow IPC.
4. Python does not depend on in-process Rust bindings.

## Quick Start

```python
from xiuxian_wendao_py import (
    WendaoTransportClient,
    WendaoTransportConfig,
    WendaoTransportEndpoint,
)

config = WendaoTransportConfig(
    endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
)
client = WendaoTransportClient(config)
print(client.preferred_modes())
print(client.flight_authority())
```

## Scope

- Transport-first package with explicit Flight and Arrow IPC connection models.
- `xiuxian_*` canonical Python entrypoint for Wendao transport consumers.
- Narrow `compat/` exports for high-traffic legacy config helpers while
  `xiuxian_foundation.*` is retired.
- Rust remains the single source of truth for search/index logic and runtime
  semantics.
