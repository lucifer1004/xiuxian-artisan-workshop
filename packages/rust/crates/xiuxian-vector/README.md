---
type: knowledge
metadata:
  title: "Omni Vector"
---

# Omni Vector

> High-Performance Embedded Vector Database using LanceDB.

## Overview

Omni Vector provides vector storage and similarity search capabilities for the Omni DevEnv. It uses LanceDB for efficient disk-based vector storage with ACID guarantees.

## Features

- Disk-based vector storage (no server required)
- Lance-backed vector similarity search
- Scanner tuning via `SearchOptions`
- CRUD + merge-insert (upsert) operations
- Versioning / snapshot (time travel) APIs
- Schema evolution helpers
- Generic Arrow IPC codec and Arrow-over-HTTP transport helpers

## Usage

```rust
use xiuxian_vector::{KeywordSearchBackend, SearchOptions, VectorStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut store = VectorStore::new("./vectors", Some(3)).await?;

    store
        .add_documents(
            "skills",
            vec!["doc1".to_string()],
            vec![vec![0.1, 0.2, 0.3]],
            vec!["example document".to_string()],
            vec![serde_json::json!({"source":"docs/readme.md"}).to_string()],
        )
        .await?;

    let results = store
        .search_optimized(
            "skills",
            vec![0.1, 0.2, 0.3],
            5,
            SearchOptions {
                where_filter: Some(r#"{"source":"docs/readme.md"}"#.to_string()),
                ..SearchOptions::default()
            },
        )
        .await?;

    println!("results={}", results.len());

    // Optional: switch keyword backend for hybrid search.
    store.set_keyword_backend(KeywordSearchBackend::LanceFts)?;
    store.create_fts_index("skills").await?;

    Ok(())
}
```

## Architecture

```
xiuxian-vector/
├── src/lib.rs                # Main exports / module wiring
├── src/arrow_transport/      # Generic Arrow IPC codec + HTTP client
├── src/ops/                  # Core CRUD + admin + writer operations
├── src/search/               # search_optimized + hybrid fusion + search_fts
├── src/keyword/              # keyword backend abstraction (Tantivy / Lance FTS)
└── tests/                    # snapshots + data-layer + perf guard
```

## Arrow Transport

`xiuxian-vector` now owns the generic Arrow IPC substrate used to speak to
external processors such as the standalone Julia `WendaoArrow` package. The
crate exposes:

- `encode_record_batch_ipc` and `decode_record_batches_ipc` for generic Arrow stream handling
- `ArrowTransportClient` for HTTP roundtrips against a WendaoArrow-compatible endpoint
- `ArrowTransportConfig::from_toml_str(...)` for `[gateway.arrow_transport]` config loading

`ArrowTransportClient` now treats `x-wendao-schema-version` as a required
response header on both `/health` and Arrow IPC responses. A missing or
mismatched schema header is treated as a protocol error.

## Arrow Ownership Boundary

`xiuxian-vector` intentionally has two Arrow surfaces:

- Lance-facing storage, mutation, and repo-hydration paths must use Lance's Arrow-57 types re-exported from `lance::deps`.
- DataFusion/search-engine execution and generic Arrow-over-HTTP transport continue to use the workspace Arrow surface.

Do not pass workspace Arrow arrays into `LanceRecordBatch` construction or downcast Lance batches using workspace Arrow collection types. Use the Lance-prefixed re-exports from `xiuxian-vector` for any code that touches Lance-owned schemas or arrays.

## Integration

Used by:

- [Skill Discovery](../../../../docs/llm/skill-discovery.md)
- [Knowledge Matrix](../../../../docs/human/architecture/knowledge-matrix.md)

## See Also

- [docs/reference/librarian.md](../../../../docs/reference/librarian.md)

## License

Apache-2.0
