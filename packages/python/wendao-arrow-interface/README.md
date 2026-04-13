---
type: knowledge
metadata:
  title: "wendao-arrow-interface"
---

# wendao-arrow-interface

`wendao-arrow-interface` is the downstream-facing Python facade for Wendao
Arrow Flight consumers.

It sits above `wendao-core-lib` and keeps one narrow goal: make the common
consumer path simple without taking transport ownership away from the existing
transport package.

Current scope for the initial slice:

1. create one `WendaoArrowSession` entrypoint for Flight-backed reads and
   exchanges
2. return one `WendaoArrowResult` object with:
   - the raw `pyarrow.Table`
   - typed row parsing helpers for stable Wendao contracts
   - Arrow-first parser hooks
   - Arrow-first analyzer hooks
3. keep typed request and row contracts sourced from `wendao-core-lib`
4. keep downstream parser and analyzer composition simple
5. expose workflow-first helpers for stable routes such as repo search,
   attachment search, and rerank
6. keep dataframe adapters such as Polars optional and example-oriented

Current testing surface:

1. `WendaoArrowResult.from_rows(...)` builds one lightweight result fixture
   from Python row dictionaries
2. `WendaoArrowSession.for_testing(...)` builds one in-memory session without
   Flight transport
3. `WendaoArrowScriptedClient` records calls so downstream tests can assert the
   exact repo-search, query, exchange, or rerank request shape plus effective
   Flight metadata
4. contract-aware helpers now exist for the stable typed workflow families:
   - `WendaoArrowSession.for_attachment_search_testing(...)`
   - `WendaoArrowSession.for_repo_search_testing(...)`
   - `WendaoArrowSession.for_rerank_response_testing(...)`
   - `WendaoArrowScriptedClient.for_attachment_search_rows(...)`
   - `WendaoArrowScriptedClient.for_repo_search_rows(...)`
   - `WendaoArrowScriptedClient.for_rerank_response_rows(...)`
5. route-scoped generic helpers also exist when you are testing a custom
   Flight route without a typed repo/rerank contract:
   - `WendaoArrowSession.for_query_testing(...)`
   - `WendaoArrowSession.for_exchange_testing(...)`
   - `WendaoArrowScriptedClient.for_query_route(...)`
   - `WendaoArrowScriptedClient.for_exchange_route(...)`
   - `WendaoArrowScriptedClient.add_query_response(...)`
   - `WendaoArrowScriptedClient.add_exchange_response(...)`
6. advanced typed tests can queue multiple typed responses through:
   - `WendaoArrowScriptedClient.add_repo_search_response(...)`
   - `WendaoArrowScriptedClient.add_attachment_search_response(...)`
   - `WendaoArrowScriptedClient.add_rerank_response(...)`
7. the facade itself re-exports `repo_search_metadata(...)`,
   `attachment_search_metadata(...)`, and `rerank_request_metadata(...)` so
   tests can assert effective Flight headers without dropping back to
   `wendao-core-lib`
8. recorded `WendaoArrowCall` values also expose `effective_metadata`,
   `derived_metadata()`, and `assert_metadata_matches_contract()` for
   contract-aware assertions that stay entirely on the facade object
9. typed scripted registrations can also accept explicit `extra_metadata=...`
   expectations, so repo, attachment, and rerank helpers can fail immediately
   when effective headers drift from the intended contract

Boundary rules:

1. `wendao-core-lib` remains the transport owner
2. Arrow remains the canonical raw interchange surface
3. Polars is an optional example adapter
4. this package does not own runtime orchestration or Flight protocol changes
5. route-specific transport helpers such as rerank stay transport/facade-owned;
   `xiuxian-wendao-analyzer` only consumes the rows or tables they return

## Quick Start

```python
from wendao_arrow_interface import WendaoArrowSession

session = WendaoArrowSession.from_endpoint(
    host="127.0.0.1",
    port=50051,
)

result = session.repo_search("rerank rust traits", limit=10)
summary = result.parse_table(lambda table: {"rows": table.num_rows})
top = result.analyze_rows(lambda rows: rows[0]["doc_id"] if rows else None)

print(summary["rows"], top)
```

## Performance Reading

The intended downstream order is:

1. Arrow Flight for transport
2. Arrow table as the canonical returned result
3. optional adapters such as Polars only after the Arrow result is already
   materialized

That keeps the transport path aligned with the Rust-owned contract while still
letting downstream consumers switch to the Python dataframe ecosystem when it
is useful.

## Examples

Shipped now:

1. [`examples/attachment_pdf_polars_workflow.py`](examples/attachment_pdf_polars_workflow.py)
   - attachment search through Arrow Flight semantics, then optional
     Arrow-to-Polars PDF analysis

Example command:

```bash
uv run --extra polars python examples/attachment_pdf_polars_workflow.py
```

## Parser and Analyzer Hooks

The result object keeps the raw Arrow table available and supports thin
consumer-side hooks:

```python
from wendao_arrow_interface import WendaoArrowSession

session = WendaoArrowSession.from_endpoint(host="127.0.0.1", port=50051)
result = session.repo_search("cache policy", limit=5)

summary = result.parse_table(lambda table: {"rows": table.num_rows})
analysis = result.analyze_rows(lambda rows: {"top_doc": rows[0]["doc_id"] if rows else None})
```

For stable Wendao transport contracts, the same result object can also reuse
typed parsers from `wendao-core-lib`:

```python
from wendao_arrow_interface import WendaoArrowSession

session = WendaoArrowSession.from_endpoint(host="127.0.0.1", port=50051)
repo_result = session.repo_search("graph cache policy", limit=5)
typed_rows = repo_result.parse_repo_search_rows()

for row in typed_rows:
    print(row.doc_id, row.path, row.score)
```

The same workflow-first shape also applies to attachment search:

```python
from wendao_arrow_interface import WendaoArrowSession

session = WendaoArrowSession.from_endpoint(host="127.0.0.1", port=50051)
attachment_result = session.attachment_search(
    "architecture",
    limit=5,
    ext_filters=("pdf",),
    kind_filters=("pdf",),
)

for row in attachment_result.parse_attachment_search_rows():
    print(row.attachment_name, row.attachment_ext, row.score)
```

If you want a pure Arrow-to-Polars consumer example for attachment search, keep
it explicit and adapter-shaped:

```python
from wendao_arrow_interface import WendaoArrowSession

session = WendaoArrowSession.for_attachment_search_testing([...])
result = session.attachment_search("architecture", ext_filters=("pdf",), kind_filters=("pdf",))
frame = result.to_polars()

print(frame.select("attachmentName", "sourceTitle", "score"))
```

That path requires the optional Polars dependency. It is not part of the
package's core Arrow-first contract.

## Testing Interface

Downstream tests do not need to monkeypatch `WendaoTransportClient` anymore:

```python
from wendao_arrow_interface import (
    WendaoArrowResult,
    WendaoArrowScriptedClient,
    WendaoArrowSession,
)

result = WendaoArrowResult.from_rows(
    [{"doc_id": "doc-1", "path": "docs/cache.md", "score": 0.9}],
    route="/search/repos/main",
)

scripted = WendaoArrowScriptedClient(repo_search_table=result.table)
session = WendaoArrowSession.from_client(scripted)
repo_result = session.repo_search("cache policy", limit=5)

assert repo_result.parse_repo_search_rows()[0].doc_id == "doc-1"
call = scripted.calls[0]
assert call.request is not None
assert call.request.query_text == "cache policy"
assert call.effective_metadata == call.derived_metadata()
call.assert_metadata_matches_contract()
```

If one typed route needs different responses across calls, queue them directly
on the scripted client:

```python
from wendao_arrow_interface import (
    WendaoArrowScriptedClient,
    WendaoArrowSession,
    attachment_search_request,
)

scripted = WendaoArrowScriptedClient()
scripted.add_attachment_search_response(
    [{"attachmentName": "design-review.pdf", "attachmentExt": "pdf", "kind": "pdf"}],
    request=attachment_search_request("architecture", ext_filters=("pdf",), kind_filters=("pdf",)),
)
scripted.add_attachment_search_response(
    [{"attachmentName": "roadmap.png", "attachmentExt": "png", "kind": "image"}],
    request=attachment_search_request("roadmap", ext_filters=("png",), kind_filters=("image",)),
)

session = WendaoArrowSession.from_client(scripted)
first = session.attachment_search("architecture", ext_filters=("pdf",), kind_filters=("pdf",))
second = session.attachment_search("roadmap", ext_filters=("png",), kind_filters=("image",))

assert first.to_rows()[0]["attachmentName"] == "design-review.pdf"
assert second.to_rows()[0]["attachmentName"] == "roadmap.png"
assert scripted.calls[0].extra_metadata is not None
```

If you want the typed fixture itself to fail when metadata drifts, pass an
explicit expectation during registration:

```python
from wendao_arrow_interface import (
    WendaoArrowScriptedClient,
    WendaoArrowSession,
    attachment_search_metadata,
    attachment_search_request,
)

request = attachment_search_request("architecture", ext_filters=("pdf",), kind_filters=("pdf",))
scripted = WendaoArrowScriptedClient().add_attachment_search_response(
    [{"attachmentName": "design-review.pdf", "attachmentExt": "pdf", "kind": "pdf"}],
    request=request,
    extra_metadata=attachment_search_metadata(request),
)

session = WendaoArrowSession.from_client(scripted)
session.attachment_search(request)
```

Custom generic routes can now be queued the same way, including expected
metadata or request-table checks:

```python
from wendao_arrow_interface import WendaoArrowScriptedClient, WendaoArrowSession
import pyarrow as pa

scripted = WendaoArrowScriptedClient()
scripted.add_query_response(
    "/search/custom/demo",
    [{"doc_id": "doc-1", "score": 0.9}],
    ticket="ticket-1",
    extra_metadata={"x-mode": "query"},
)
scripted.add_exchange_response(
    "/exchange/custom/demo",
    [{"doc_id": "doc-2", "status": "ok"}],
    ticket="ticket-2",
    extra_metadata={"x-mode": "exchange"},
    request_table=[{"seed": "value"}],
)

session = WendaoArrowSession.from_client(scripted)
query = session.query("/search/custom/demo", ticket="ticket-1", extra_metadata={"x-mode": "query"})
exchange = session.exchange(
    "/exchange/custom/demo",
    pa.Table.from_pylist([{"seed": "value"}]),
    ticket="ticket-2",
    extra_metadata={"x-mode": "exchange"},
)

assert query.to_rows()[0]["doc_id"] == "doc-1"
assert exchange.to_rows()[0]["status"] == "ok"
```

The live generic session surface also accepts `WendaoFlightRouteQuery`
directly, so callers that already have a typed query object do not need to
reconstruct `route + ticket` manually:

```python
from wendao_arrow_interface import WendaoArrowSession, WendaoFlightRouteQuery

session = WendaoArrowSession.from_endpoint(host="127.0.0.1", port=50051)
query = WendaoFlightRouteQuery(route="/search/custom/demo", ticket="custom-ticket")

result = session.query(query, extra_metadata={"x-mode": "query"})
```

For the stable repo-search and rerank workflows, prefer the contract-aware
helpers over raw route registration:

```python
from wendao_arrow_interface import WendaoArrowSession

session = WendaoArrowSession.for_repo_search_testing(
    [{"doc_id": "doc-1", "path": "docs/cache.md", "score": 0.9}]
)

repo_result = session.repo_search("cache policy", limit=5)
query_result = session.query("/search/repos/main")

assert repo_result.parse_repo_search_rows()[0].doc_id == "doc-1"
assert query_result.parse_repo_search_rows()[0].doc_id == "doc-1"
```

Attachment search has the same contract-aware testing helper:

```python
from wendao_arrow_interface import WendaoArrowSession

session = WendaoArrowSession.for_attachment_search_testing(
    [
        {
            "name": "Architecture PDF",
            "path": "notes/architecture.md#attachments/design-review.pdf",
            "sourceId": "doc-attachment-1",
            "sourceStem": "architecture",
            "sourceTitle": "Architecture Notes",
            "navigationTargetJson": '{"kind":"note","path":"notes/architecture.md"}',
            "sourcePath": "notes/architecture.md",
            "attachmentId": "attachment-1",
            "attachmentPath": "assets/design-review.pdf",
            "attachmentName": "design-review.pdf",
            "attachmentExt": "pdf",
            "kind": "pdf",
            "score": 0.82,
            "visionSnippet": "System design overview",
        }
    ]
)

result = session.attachment_search("architecture", ext_filters=("pdf",), kind_filters=("pdf",))
assert result.parse_attachment_search_rows()[0].attachment_name == "design-review.pdf"
```

For simpler tests that only need static responses, use the convenience
constructor:

```python
from wendao_arrow_interface import WendaoArrowSession

session = WendaoArrowSession.for_testing(
    query_tables={
        "/search/repos/main": [
            {"doc_id": "doc-1", "path": "docs/cache.md", "score": 0.9}
        ]
    }
)

result = session.query("/search/repos/main")
assert result.to_rows()[0]["doc_id"] == "doc-1"
```

If you are testing a custom generic route, prefer the route-scoped helper over
manually building `query_tables` or `exchange_tables`:

```python
from wendao_arrow_interface import WendaoArrowSession

session = WendaoArrowSession.for_query_testing(
    "/search/custom/demo",
    [{"doc_id": "doc-1", "score": 0.9}],
)

result = session.query("/search/custom/demo")
assert result.query is not None
assert result.query.normalized_route() == "/search/custom/demo"
```

## Initial Public Surface

1. `connect(...)`
2. `WendaoArrowSession`
3. `WendaoArrowResult`
4. parser and analyzer protocols for Arrow tables and row analyzers
5. selected typed Wendao request and row contracts re-exported from
   `wendao-core-lib`
6. one in-memory testing surface through `WendaoArrowSession.for_testing(...)`,
   route-scoped generic helpers, contract-aware attachment-search/repo-search/rerank helpers,
   `WendaoArrowResult.from_rows(...)`, and `WendaoArrowScriptedClient`
