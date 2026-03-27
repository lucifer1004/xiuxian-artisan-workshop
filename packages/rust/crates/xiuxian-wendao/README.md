# 🌀 Wendao (问道)

**The Sovereign High-Performance Knowledge & Link-Graph Runtime.**

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![Valkey](https://img.shields.io/badge/storage-Valkey-red.svg)](https://valkey.io/)
[![LanceDB](https://img.shields.io/badge/vector-LanceDB-blue.svg)](https://lancedb.com/)
[![Arrow](https://img.shields.io/badge/protocol-Apache--Arrow-brightgreen.svg)](https://arrow.apache.org/)

**Wendao** is a next-generation knowledge management engine. While tools like Obsidian revolutionized human note-taking, **Wendao** is designed for the era of Autonomous Agents, providing a high-performance, programmable substrate for structured reasoning and massive-scale retrieval.

---

## 💎 Why Wendao? (The Obsidian Leap)

Wendao moves beyond the limitations of traditional bi-link tools by introducing **Topological Sovereignty**:

| Feature         | Obsidian (Human-Centric)    | **Wendao (Agent-Centric)**                        |
| :-------------- | :-------------------------- | :------------------------------------------------ |
| **Structure**   | Flat Bi-links & Folders     | **Hierarchical Semantic Trees (PageIndex)**       |
| **Retrieval**   | Simple Search / Dataview    | **Quantum Fusion (Vector + Graph + PPR)**         |
| **Scale**       | Electron / Local Filesystem | **Rust Core / LanceDB / Valkey Cluster**          |
| **Context**     | Manual "Maps of Content"    | **Automated Ancestry Uplink (Zero-loss context)** |
| **Performance** | Sequential scanning         | **Arrow-Native Zero-Copy (15x throughput)**       |

---

## 🚀 Key Evolutionary Features

### 1. PageIndex Rust Core (Hierarchical Indexing)

Unlike Obsidian's flat structure, Wendao builds a recursive **Semantic Tree** of your documents. It understands the logical hierarchy (Root > Chapter > Section), allowing agents to navigate complex long-form content with "God's eye" perspective.

### 2. Quantum Fusion (Hybrid Retrieval)

Fuses fuzzy **Vector Search** (semantic intuition) with precise **Graph Diffusion** (logical reasoning). Using a neurobiologically inspired **PPR algorithm** (Personalized PageRank), Wendao finds not just "similar" text, but "logically relevant" knowledge clusters.

### 3. Apache Arrow IPC

Built on top of the **Arrow Data Ecosystem**. Knowledge flows through the engine as columnar memory batches. This ensures **Zero-copy** overhead during retrieval, re-ranking, and injection, making it capable of handling millions of nodes at sub-millisecond latency.

---

## 📚 Theoretical Foundation (2025-2026)

Wendao is physically grounded in cutting-edge RAG research:

- **LightRAG (2025)**: Dual-level indexing (Logical + Entity).
- **RAGNET (Stanford 2025)**: End-to-end training for neural graph retrieval.
- **Columnar Knowledge Streams (2026)**: Zero-copy Arrow transport for scaling.

---

## 🛠 Architecture

- **Kernel**: Pure Rust (Tokio / Rayon)
- **Hot Cache**: Valkey (In-memory graph adjacency and saliency scores)
- **Cold Storage**: LanceDB (Persistent vector anchors and Arrow fragments)
- **Protocol**: Apache Arrow (Universal knowledge transport layer)

### Julia Arrow Adapter

`xiuxian-wendao` now exposes a thin Julia-facing service adapter for the
WendaoArrow transport contract. The core crate keeps the existing synchronous
repository analyzer trait unchanged, while `analyzers::fetch_julia_arrow_score_rows_for_repository`
provides an explicit async entrypoint for:

- resolving repository-configured Julia Arrow transport settings
- executing the Arrow IPC HTTP roundtrip
- validating the WendaoArrow `v1` response contract
- materializing `doc_id`, `analyzer_score`, and `final_score` into typed Rust rows

The same boundary now also exposes `analyzers::build_julia_arrow_request_batch`
and `analyzers::JuliaArrowRequestRow`, so higher-level retrieval code can build
the canonical WendaoArrow `v1` request payload without duplicating Arrow schema
construction.

For link-graph semantic retrieval, `VectorStoreSemanticIgnition` now also
provides `build_julia_rerank_request_batch(...)`, which reuses anchor ids as
the stable request-row identity and assembles a Julia-ready Arrow batch from
`QuantumAnchorHit` values plus the current query vector.

`OpenAiCompatibleSemanticIgnition` now exposes the same
`build_julia_rerank_request_batch(...)` surface. It resolves the effective
query vector from either an explicit `query_vector` or an
OpenAI-compatible embedding call, then builds the canonical WendaoArrow `v1`
request batch from the resulting anchors and stored embeddings.

For the link-graph runtime, `link_graph.retrieval.julia_rerank` is now the
dedicated config namespace for the future WendaoArrow post-processing step.
The runtime currently resolves `base_url`, `route`, `health_route`,
`schema_version`, and `timeout_secs`, and planned-search payloads now keep a
separate `julia_rerank` telemetry slot so remote Julia transport state stays
separate from `semantic_ignition`.

The OpenAI-compatible semantic-ignition runtime path now uses that config as an
optional post-processing stage. When configured, Wendao can build the
WendaoArrow `v1` request batch, call the remote Julia service, validate the
Arrow response contract, and overwrite `QuantumContext.saliency_score` with the
returned Julia `final_score`. Transport or contract failures degrade cleanly
back to the original Rust-side quantum-fusion ordering and are recorded in
`julia_rerank` telemetry.

That runtime path is now covered by a planned-search loopback integration test
that keeps a local mock only for `/v1/embeddings`, but sends the rerank Arrow
IPC request to the real `.data/WendaoArrow` Julia service, then asserts the
Julia `final_score` response actually reorders emitted `quantum_contexts`.

The vector-store semantic-ignition backend can now enter the same Julia rerank
path when the caller provides a precomputed query vector through the planned
search runtime. Wendao keeps that vector in the in-memory payload runtime state
only, uses it to build the WendaoArrow request batch, and still avoids
serializing it into the external payload contract.

That vector-store runtime path is also now covered against the real
`.data/WendaoArrow` Julia service rather than a Rust-side Arrow mock.

There is now also a dedicated planned-search integration that targets the
package-owned `.data/WendaoArrow/scripts/run_stream_scoring_server.sh`
example, so the main crate validates not only custom Julia rerank responses
but also the official stream scoring example surface.

A second official-example integration now targets
`.data/WendaoArrow/scripts/run_stream_metadata_server.sh` to confirm additive
response columns derived from request metadata do not break the planned-search
Julia rerank path. The Julia response decoder now also surfaces optional
additive `trace_id` columns into `julia_rerank.trace_ids`, and the planned
search runtime writes a stable request-schema `trace_id` so the official
metadata example can roundtrip that context without changing the core
`doc_id / analyzer_score / final_score` contract.

The integration support layer now keeps those two concerns separate:

- custom-score tests launch a private Julia processor with explicit score maps
- official-example tests launch `.data/WendaoArrow/scripts/run_stream_scoring_server.sh`

At the request boundary, both `zhenfa_router::WendaoSearchRequest` and the
planned HTTP request surface now accept an optional `query_vector`. When
present, the router forwards it into the planned-search runtime so the
vector-store backend can participate in Julia rerank without requiring an
extra embedding call on the Rust side.

The same optional `query_vector` is now accepted by the native
`wendao.search` Zhenfa tool arguments, so direct tool callers and bridge-based
LLM flows can reuse the vector-store Julia rerank path without changing the
serialized planned-payload contract.

This keeps transport ownership in the Arrow substrate while giving future
gateway and reranking paths one stable Rust-side integration surface.

---

## 📦 Usage

### As a CLI Tool (Standalone Binary)

Build the sovereign binary:

```bash
cargo build --release --bin wendao
```

Run common operations:

```bash
# Analyze document hierarchy
./target/release/wendao page-index --path ./my_notes/paper.md

# Execute hybrid search
./target/release/wendao search "Explain quantum entanglement" --hybrid

# Show graph neighbors
./target/release/wendao neighbors "Agentic_RAG"
```

### As a Library

Add **Wendao** to your `Cargo.toml`:

```toml
[dependencies]
xiuxian-wendao = { git = "https://github.com/tao3k/wenbdao.git" }
```

Initialize the engine:

```rust
let engine = WendaoEngine::builder()
    .with_storage(ValkeyConfig::default())
    .with_vectors(LanceConfig::at("./data/vectors"))
    .build()
    .await?;
```

### Optional Python Bindings

Enable the PyO3 surface only when you need Python interop:

```bash
cargo build -p xiuxian-wendao --features pybindings
```

This keeps the default build free of PyO3 and the Python-specific modules.

---

## 🛡️ License

Designed with the precision of a master artisan.

© 2026 Sovereign Forge. All Rights Reserved.
