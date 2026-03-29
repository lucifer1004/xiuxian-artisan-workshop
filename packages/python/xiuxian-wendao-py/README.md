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
2. Python plugin authors implement analyzers against Arrow tables.
3. Python prefers Arrow Flight transport.
4. Python falls back to Arrow IPC.
5. Python does not depend on in-process Rust bindings.

## Quick Start

```python
from xiuxian_wendao_py import (
    run_analyzer,
    WendaoAnalyzerPlugin,
    WendaoTransportClient,
    WendaoTransportConfig,
    WendaoTransportEndpoint,
    WendaoFlightRouteQuery,
)

config = WendaoTransportConfig(
    endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
)
client = WendaoTransportClient(config)
query = WendaoFlightRouteQuery(route="/search/repos/main")

def analyzer(table, context):
    return {
        "rows": table.num_rows,
        "route": context.query.normalized_route(),
        "flight_info": context.flight_info,
    }

result = run_analyzer(client, analyzer, query)
print(result)
```

## Analyzer Plugin Scaffold

```python
from xiuxian_wendao_py import (
    WendaoAnalyzerPlugin,
    WendaoFlightRouteQuery,
    WendaoTransportClient,
    WendaoTransportConfig,
    WendaoTransportEndpoint,
)

client = WendaoTransportClient(
    WendaoTransportConfig(
        endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
    )
)
query = WendaoFlightRouteQuery(route="/search/repos/main")

plugin = WendaoAnalyzerPlugin(
    capability_id="repo_search",
    provider="acme.python.analyzer",
    analyzer=lambda table, context: {
        "rows": table.num_rows,
        "route": context.query.normalized_route(),
    },
)

binding = plugin.binding_for_client(client, query)
result = plugin.run(client, query)
print(binding.to_dict())
print(result)
```

## Scope

- Arrow-backed analyzer authoring helpers that keep Flight descriptor, metadata,
  and table-fetch plumbing out of downstream analyzer code.
- Plugin-binding scaffold aligned to the Rust capability/endpoint/transport
  contract so downstream Python providers do not need to hand-build runtime
  binding payloads.
- Transport-first package with explicit Flight connection setup and Arrow IPC fallback models.
- `xiuxian_*` canonical Python entrypoint for Wendao transport consumers and plugin authors.
- Rust remains the single source of truth for search/index logic and runtime
  semantics.
