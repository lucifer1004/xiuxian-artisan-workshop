---
title: "Vector Index Optimization"
category: "explanation"
tags:
  - explanation
  - vector
saliency_base: 6.0
decay_rate: 0.04
---

# Vector Index Optimization

> Foundation Layer - LanceDB-based Semantic Search

## Overview

The vector index provides fast nearest-neighbor (ANN) search for semantic tool discovery and knowledge retrieval. It uses LanceDB with adaptive IVF-FLAT indexing.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ Python Layer (omni.foundation.vector_store)                 │
│  - VectorStoreClient singleton                              │
│  - Async search/add/delete operations                       │
│  - Collection management                                    │
└────────────────────────┬────────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────────┐
│ Rust Bindings (omni-core-rs)                                │
│  - create_vector_store() factory                            │
│  - PyVectorStore wrapper                                    │
└────────────────────────┬────────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────────┐
│ Rust Core (omni-vector crate)                               │
│  - VectorStore: LanceDB operations                          │
│  - SkillScanner: Tool discovery                             │
│  - ScriptScanner: @skill_command detection                  │
└─────────────────────────────────────────────────────────────┘
```

## Adaptive Index Strategy

The index uses adaptive partitioning based on dataset size:

```rust
// packages/rust/crates/omni-vector/src/index.rs

const MIN_VECTORS_FOR_INDEX: usize = 100;
const VECTORS_PER_PARTITION: usize = 256;
const MAX_PARTITIONS: usize = 512;

let num_partitions = (num_rows / VECTORS_PER_PARTITION).clamp(32, 512);
```

### Partition Table

| Dataset Size   | Partitions | Behavior                    |
| -------------- | ---------- | --------------------------- |
| < 100 vectors  | Skip       | Flat search is faster       |
| 100 - 5,000    | 32         | Small dataset optimization  |
| 5,000 - 50,000 | 20 - 196   | Balanced recall/performance |
| > 50,000       | 512        | Avoid over-sharding         |

## Search Optimization

```rust
// packages/rust/crates/omni-vector/src/search.rs

const FETCH_MULTIPLIER: usize = 2;

let fetch_count = limit.saturating_mul(2).max(limit + 10);
```

The search fetches 2× the requested limit to account for metadata filtering loss.

## Hybrid Search

Combines vector similarity with keyword boosting:

```rust
// Formula: Score = Vector_Score * 0.7 + Keyword_Match * 0.3
```

| Match Type                 | Boost |
| -------------------------- | ----- |
| Metadata keywords          | +0.1  |
| Tool name contains keyword | +0.05 |
| Content contains keyword   | +0.03 |

## API Usage

### Python (Foundation Layer)

```python
from omni.foundation.vector_store import get_vector_store

# Get singleton client
store = get_vector_store()

# Search
results = await store.search("git commit workflow", n_results=5)

# Add content
await store.add(
    content="Execute git commit with message",
    metadata={"skill": "git", "command": "commit"},
    collection="skills"
)

# Create index
await store.create_index("skills")
```

### Configuration

```yaml
# settings (system: packages/conf/settings.yaml, user: $PRJ_CONFIG_HOME/omni-dev-fusion/settings.yaml)
vector:
  path: ".cache/omni-vector"
  dimension: 1536 # OpenAI Ada-002
  default_limit: 5
```

## Performance Characteristics

| Operation    | Time Complexity | Notes                   |
| ------------ | --------------- | ----------------------- |
| Search       | O(log n + k)    | ANN with IVF index      |
| Add          | O(d)            | Single vector insertion |
| Create Index | O(n log n)      | Batch index build       |

## Scalar Indices (Phase 1)

BTree and Bitmap indices on metadata columns (`skill_name`, `category`) for faster filters:

- **Location:** `packages/rust/crates/omni-vector/src/ops/scalar.rs`
- **APIs:** `create_btree_index`, `create_bitmap_index`, `create_optimal_scalar_index` (cardinality &lt; 100 → Bitmap, else BTree). Skill index write triggers best-effort scalar index creation.
- **Roadmap:** [LanceDB Version and Roadmap](../reference/lancedb-version-and-roadmap.md).

## Auto-Indexing and Maintenance (Phase 2)

- **Location:** `packages/rust/crates/omni-vector/src/ops/maintenance.rs`
- **APIs:** `has_vector_index`, `has_fts_index`, `has_scalar_index`; `auto_index_if_needed` / `auto_index_if_needed_with_thresholds`; `compact(table_name)` (cleanup + compact_files).
- **Thresholds:** Configurable via `IndexThresholds` (e.g. `auto_index_at` row count).

## Vector Index Tuning (Phase 3)

- **Location:** `packages/rust/crates/omni-vector/src/ops/vector_index.rs`
- **APIs:** `create_hnsw_index` (IVF+HNSW for smaller tables), `create_optimal_vector_index` (HNSW &lt; 10k rows, IVF_FLAT ≥ 10k).

## Partitioning Suggestions (Phase 4)

- **Location:** `packages/rust/crates/omni-vector/src/ops/partitioning.rs`
- **APIs:** `suggest_partition_column(table_name)` returns a suggested column (e.g. `skill_name`) when the table has ≥ 10k rows and a partition-friendly schema. Wired into health report as `Recommendation::Partition { column }`.

## Observability (Phase 5)

- **Location:** `packages/rust/crates/omni-vector/src/ops/observability.rs`
- **APIs:** `analyze_table_health(table_name)` → `TableHealthReport` (row_count, fragment_count, fragmentation_ratio, indices_status, recommendations); `get_query_metrics(table_name)` → `QueryMetrics` (placeholder for future Lance tracing).
- **Types:** `IndexStatus`, `Recommendation` (e.g. `RunCompaction`, `CreateIndices`, `Partition { column }`), `TableHealthReport`, `QueryMetrics`.

## Related Files

**Python:**

- `packages/python/foundation/src/omni/foundation/services/vector.py`

**Rust:**

- `packages/rust/crates/omni-vector/src/lib.rs`
- `packages/rust/crates/omni-vector/src/index.rs`
- `packages/rust/crates/omni-vector/src/search/` (search_impl, options)
- `packages/rust/crates/omni-vector/src/ops/` (admin_impl, writer_impl, maintenance, scalar, vector_index, observability, partitioning, types)

**Bindings:**

- `packages/rust/bindings/python/src/vector.rs`

**Roadmap:**

- [LanceDB Version and Roadmap](../reference/lancedb-version-and-roadmap.md)

---

title: "Vector Checkpoint System"
category: "explanation"
tags:

- explanation
- vector
- checkpoint
- checkpoint-schema
  saliency_base: 6.0
  decay_rate: 0.04

---

# Checkpoint Schema and Vector Checkpoint System

> Agent Layer - LangGraph Checkpoint Persistence with Semantic Search

## Overview

The vector checkpoint system provides state persistence for LangGraph workflows using LanceDB. It combines traditional checkpoint storage with semantic search capabilities, enabling experience recall across sessions.

This document is the primary reference for the query phrase `checkpoint schema`.

## 2026-02 Update (Current Baseline)

### Checkpoint Schema (Primary Query Anchor)

For query phrase `checkpoint schema`, the canonical contract is:

- `packages/rust/crates/omni-vector/resources/omni.checkpoint.record.v1.schema.json`

This schema defines the checkpoint record validation boundary shared by Python API and Rust runtime.

### Shared Schema Contract (Single Source of Truth)

- Checkpoint payload schema is now centralized at:
  - `packages/rust/crates/omni-vector/resources/omni.checkpoint.record.v1.schema.json`
- Python side validates through:
  - `omni.foundation.api.checkpoint_schema`
- Rust binding side validates against the same JSON schema before write:
  - `packages/rust/bindings/python/src/checkpoint.rs`

### Strict Validation (No Silent Fallback)

- Schema validation is mandatory for checkpoint writes.
- Checkpoint runtime is Rust-only (`omni_core_rs`); legacy SQLite/in-memory fallback path is removed.
- Missing schema file now fails fast (`FileNotFoundError`) instead of silently skipping validation.
- Semantic constraints are also enforced:
  - `table_name` must be non-empty
  - `parent_id != checkpoint_id`
  - timestamp must be finite
  - embedding values must be finite
  - metadata must decode to JSON object string

### Rust Core Auto-Repair

`omni-vector` checkpoint store now includes startup self-healing:

- Schema drift detection and repair (`validate_dataset_schema`)
- Startup repair guard (`run_startup_repairs_once`)
- Interrupted/orphan state cleanup (including dangling parent chains)

Store implementation was split into focused modules for maintainability:

- `src/checkpoint/store/lifecycle.rs`
- `src/checkpoint/store/schema.rs`
- `src/checkpoint/store/maintenance.rs`
- `src/checkpoint/store/read_ops.rs`
- `src/checkpoint/store/write_ops.rs`
- `src/checkpoint/store/search_ops.rs`
- `src/checkpoint/store/timeline_ops.rs`

## Linked Notes

- Related: [Vector Store API](../reference/vector-store-api.md)
- Related: [Vector/Router Schema Contract](../reference/vector-router-schema-contract.md)

```text
┌─────────────────────────────────────────────────────────────────────────┐
│ Python Layer (omni.langgraph.checkpoint)                                │
│  ┌────────────────────────┐  ┌─────────────────────────────────────────┐│
│  │ LanceCheckpointer      │  │ RustCheckpointSaver (LangGraph adapter) ││
│  │ - Core checkpoint ops  │  │ - BaseCheckpointSaver interface         ││
│  │ - Semantic search      │  │ - CheckpointTuple NamedTuple returns    ││
│  │ - Rust LanceDB bridge  │  │ - Async method delegation               ││
│  └───────────┬────────────┘  └────────────────┬────────────────────────┘│
└──────────────┼─────────────────────────────────┼────────────────────────┘
               │                                 │
┌──────────────▼─────────────────────────────────▼────────────────────────┐
│ Rust Bindings (omni-core-rs)                                              │
│  - create_checkpoint_store() factory                                     │
│  - CheckpointStore wrapper                                               │
│  - LanceDB operations via PyO3                                           │
└────────────────────────┬────────────────────────────────────────────────┘
                         │
┌────────────────────────▼────────────────────────────────────────────────┐
│ Rust Core (omni-vector crate)                                            │
│  - CheckpointStore: LanceDB operations                                   │
│  - Semantic search with embeddings                                       │
│  - Parent checkpoint chains                                              │
└─────────────────────────────────────────────────────────────────────────┘
```

## Key Concepts

### Checkpoint (LangGraph 1.0+ Format)

A checkpoint captures the complete state of a LangGraph workflow:

```python
{
    "v": 2,                         # Checkpoint version (LangGraph 1.0+)
    "id": "abc123hex",              # UUID6 hex (time-ordered)
    "ts": "2024-01-01T00:00:00Z",   # ISO 8601 timestamp
    "channel_values": {...},        # Workflow state (LangGraph 1.0+)
    "channel_versions": {},         # Per-channel version tracking
    "versions_seen": {},            # Node-level version tracking
    "updated_channels": None,       # List of updated channel names
}
```

### CheckpointTuple

LangGraph uses `CheckpointTuple` NamedTuple for checkpoint operations:

```python
from langgraph.checkpoint.base import CheckpointTuple

CheckpointTuple(
    config={"configurable": {"thread_id": "..."}},  # RunnableConfig
    checkpoint={...},                               # Checkpoint dict
    metadata={"source": "input", "step": 0, "writes": {}},
    parent_config=None,                             # Optional parent
    pending_writes=None,                            # Optional pending writes
)
```

### Thread ID

Each workflow session has a unique thread ID for checkpoint isolation:

```python
config = {"configurable": {"thread_id": "research-session-123"}}
```

### Parent Checkpoint Chain

Checkpoints form a history chain via parent links, enabling full state reconstruction:

```text
checkpoint_3 (parent: checkpoint_2)
    └── checkpoint_2 (parent: checkpoint_1)
            └── checkpoint_1 (parent: None)
                    └── Initial State
```

## Architecture

### LanceCheckpointer

Low-level checkpoint operations with semantic search:

```python
from omni.langgraph.checkpoint.lance import LanceCheckpointer

checkpointer = LanceCheckpointer(
    uri=".cache/checkpoints.lance",  # Optional: auto-generated
    dimension=1536,                   # Embedding dimension (OpenAI Ada-002)
)

# Save checkpoint
checkpoint_id = checkpointer.put(
    thread_id="session-123",
    state={"current_plan": "Fix bug in auth", "step": 2},
    checkpoint_id="cp-001",           # Optional: auto-generated UUID6
    metadata={"source": "user_input"}, # Optional metadata
)

# Get latest checkpoint
state = checkpointer.get("session-123")

# Get checkpoint history
history = checkpointer.get_history("session-123", limit=10)

# Delete all checkpoints for a thread
count = checkpointer.delete("session-123")

# Semantic search for similar states
results = checkpointer.search_similar(
    query_vector=embedding,  # 1536-dim vector
    thread_id=None,          # Optional: search specific thread
    limit=5,
    filter_metadata={"success": True},  # Optional filter
)
```

### RustCheckpointSaver

LangGraph-compatible adapter implementing `BaseCheckpointSaver`:

```python
from omni.langgraph.checkpoint.saver import RustCheckpointSaver
from langgraph.graph import StateGraph

# Create saver with custom settings
saver = RustCheckpointSaver(
    table_name="checkpoints",       # Table name for isolation
    uri=".cache/checkpoints.lance", # LanceDB path
    dimension=1536,                 # Embedding dimension
)

# Use with LangGraph
workflow = StateGraph(GraphState)
# ... add nodes and edges ...
app = workflow.compile(checkpointer=saver)

# Run workflow with checkpoints
result = app.invoke(
    initial_state,
    config={"configurable": {"thread_id": "session-123"}}
)
```

#### Interface Methods

The `RustCheckpointSaver` implements the `BaseCheckpointSaver` interface:

| Method                                            | Type  | Description                             |
| ------------------------------------------------- | ----- | --------------------------------------- |
| `get_tuple(config)`                               | sync  | Get latest checkpoint for thread        |
| `put(config, checkpoint, metadata, new_versions)` | sync  | Save checkpoint                         |
| `list(config, limit, filter, before)`             | sync  | List checkpoint history                 |
| `delete_thread(thread_id)`                        | sync  | Delete all checkpoints                  |
| `aget_tuple(config)`                              | async | Async version of get_tuple              |
| `aput(...)`                                       | async | Async version of put                    |
| `alist(...)`                                      | async | Async version of list (async generator) |
| `adelete_thread(thread_id)`                       | async | Async version of delete_thread          |

**Note:** Async methods delegate to sync implementations (LangGraph 1.0+ pattern).

## Semantic Search & Experience Recall

### How It Works

When the system saves a checkpoint, it performs these actions:

1. Serialize the workflow state to JSON
2. Extract searchable text from the state (prioritizes `current_plan` field)
3. Generate an embedding vector using the embedding service
4. Store the embedding alongside the checkpoint in LanceDB

This enables finding semantically similar historical states for experience recall:

```python
# Recall similar successful solutions
from omni.langgraph.checkpoint.lance import LanceCheckpointer

checkpointer = LanceCheckpointer()

# Find similar successful states
similar = checkpointer.search_similar(
    query_vector=current_state_embedding,
    limit=3,
    filter_metadata={"outcome": "success"},
)

for content, metadata, distance in similar:
    print(f"Similarity: {distance:.3f}")
    print(f"State: {content}")
```

### Embedding Generation

The system extracts text from checkpoints for embedding:

```python
# Priority: current_plan field > full state JSON
search_text = state.get("current_plan", "") or json.dumps(state)
```

## Data Model

### LanceDB Schema

| Column     | Type                   | Description                                |
| ---------- | ---------------------- | ------------------------------------------ |
| `id`       | String                 | Unique checkpoint ID                       |
| `vector`   | FixedSizeList<Float32> | State embedding (1536-dim)                 |
| `content`  | String                 | Serialized state JSON                      |
| `metadata` | String                 | JSON metadata (thread_id, timestamp, etc.) |

### CheckpointRecord (Rust)

```rust
pub struct CheckpointRecord {
    pub checkpoint_id: String,      // Unique ID
    pub thread_id: String,          // Session identifier
    pub parent_id: Option<String>,  // Parent checkpoint ID
    pub timestamp: f64,             // Unix timestamp
    pub content: String,            // JSON serialized state
    pub embedding: Option<Vec<f32>>, // Semantic embedding
    pub metadata: Option<String>,   // JSON metadata
}
```

## Configuration

```yaml
# settings (system: packages/conf/settings.yaml, user: $PRJ_CONFIG_HOME/omni-dev-fusion/settings.yaml)
checkpoint:
  path: ".cache/checkpoints.lance"
  dimension: 1536 # OpenAI Ada-002 embedding dimension
  default_limit: 10
```

## Integration Points

### With LangGraph

```python
from omni.langgraph.checkpoint.saver import RustCheckpointSaver
from langgraph.graph import END, StateGraph

# Define state
class WorkflowState(TypedDict):
    messages: Annotated[list[str], operator.add]
    current_plan: str

# Build graph
workflow = StateGraph(WorkflowState)
workflow.add_node("plan", plan_node)
workflow.add_node("execute", execute_node)
workflow.set_entry_point("plan")
workflow.add_edge("plan", "execute")
workflow.add_edge("execute", END)

# Compile with checkpoint saver
app = workflow.compile(
    checkpointer=RustCheckpointSaver()
)

# Run with thread ID
result = app.invoke(
    {"messages": ["Hello"], "current_plan": ""},
    config={"configurable": {"thread_id": "user-session-001"}}
)
```

### With Skills

```python
# assets/skills/git/scripts/smart_commit_workflow.py
from omni.foundation.config.logging import get_logger
from omni.langgraph.checkpoint.saver import RustCheckpointSaver

logger = get_logger("git.smart_commit")

# Import Rust checkpoint saver
try:
    from omni.langgraph.checkpoint.saver import RustCheckpointSaver as _RustCheckpointSaver
    _CHECKPOINT_AVAILABLE = True
    logger.info("RustCheckpointSaver imported successfully")
except ImportError as e:
    _CHECKPOINT_AVAILABLE = False
    _RustCheckpointSaver = None  # type: ignore
    logger.warning(f"RustCheckpointSaver import failed: {e}")

# Compile with checkpointer at module level
if _CHECKPOINT_AVAILABLE and _RustCheckpointSaver:
    try:
        _memory = _RustCheckpointSaver()
        logger.info(f"RustCheckpointSaver initialized: {_memory}")
    except Exception as e:
        logger.error(f"RustCheckpointSaver init failed: {e}")
        _memory = None
else:
    _memory = None

_app = create_sharded_research_graph().compile(checkpointer=_memory)
logger.info(f"Compiled app checkpointer: {_app.checkpointer}")
```

## Performance

| Operation       | Time Complexity | Notes                                 |
| --------------- | --------------- | ------------------------------------- |
| Put Checkpoint  | O(d)            | Single vector insertion               |
| Get Latest      | O(n)            | Linear scan with timestamp comparison |
| Get History     | O(n log n)      | Sort by timestamp                     |
| Semantic Search | O(log n + k)    | ANN with vector index                 |
| Delete Thread   | O(n)            | Linear scan + delete                  |

## Related Files

**Python:**

- `packages/python/agent/src/omni/langgraph/checkpoint/lance.py` - LanceCheckpointer
- `packages/python/agent/src/omni/langgraph/checkpoint/saver.py` - RustCheckpointSaver
- `packages/python/agent/src/omni/langgraph/checkpoint/__init__.py` - Exports
- `assets/skills/researcher/tests/test_researcher.py` - Integration tests

**Rust:**

- `packages/rust/crates/omni-vector/src/checkpoint.rs` - CheckpointStore
- `packages/rust/crates/omni-vector/src/lib.rs` - Main lib
- `packages/rust/bindings/python/src/vector.rs` - PyO3 bindings

**Configuration:**

- Merged settings - Runtime configuration: system `packages/conf/settings.yaml`, user `$PRJ_CONFIG_HOME/omni-dev-fusion/settings.yaml`

---

## Event-Driven Checkpointing (v5.0)

**Location**: `packages/python/core/src/omni/core/services/persistence.py`

The `AsyncPersistenceService` provides fire-and-forget checkpoint saving via the Rust Event Bus.

### Architecture

```
OmniLoop._publish_step_complete()
              ↓
PyGlobalEventBus.publish("agent", "agent/step_complete", payload)
              ↓
KernelReactor (Python async consumer)
              ↓
AsyncPersistenceService.handle_agent_step()
              ↓
Background Worker (async queue)
              ↓
Rust CheckpointStore.save_checkpoint()
```

### Usage

```python
from omni.core.services.persistence import AsyncPersistenceService
from omni.core.kernel.reactor import get_reactor

# Create service with Rust store wrapper
service = AsyncPersistenceService(rust_store)

# Register handler with reactor
reactor = get_reactor()
reactor.register_handler("agent/step_complete", service.handle_agent_step)

# Start service
await service.start()

# Service runs in background - checkpoints queued and saved asynchronously
```

### Service Methods

| Method                     | Description                             |
| -------------------------- | --------------------------------------- |
| `start()`                  | Start background save worker            |
| `stop()`                   | Stop worker and flush pending saves     |
| `handle_agent_step(event)` | Event handler for `agent/step_complete` |
| `is_running`               | Check if service is active              |
| `get_queue_size()`         | Get pending saves count                 |

### Integration with Agent Loop

The agent loop publishes checkpoint events:

```python
# In omni/agent/core/omni/loop.py
try:
    from omni_core_rs import PyGlobalEventBus
    EVENT_BUS_AVAILABLE = True
except ImportError:
    EVENT_BUS_AVAILABLE = False

def _publish_step_complete(self, state: Dict[str, Any]) -> None:
    """Fire-and-forget checkpoint event to Rust Event Bus."""
    if not EVENT_BUS_AVAILABLE:
        return

    self.current_step += 1
    payload = json.dumps({
        "thread_id": self.session_id,
        "step": self.current_step,
        "state": state,
        "timestamp": time.time(),
    })

    PyGlobalEventBus.publish("agent", "agent/step_complete", payload)
```

---

title: "Omni-Vector Project Status"
category: "references"
tags:

- reference
- omni
  saliency_base: 5.5
  decay_rate: 0.05

---

# Omni-Vector Project Status

> Feature matrix and gap list aligned with the codebase. Used for planning Python API exposure and CLI extension.

---

## 1. Completed Features

### 1.1 Rust Core (omni-vector)

| Module         | File                                    | Features                                                                           | Status |
| -------------- | --------------------------------------- | ---------------------------------------------------------------------------------- | ------ |
| Scalar Indices | `ops/scalar.rs`                         | `create_btree_index`, `create_bitmap_index`, `create_optimal_scalar_index`         | Done   |
| Vector Indices | `ops/vector_index.rs`                   | `create_hnsw_index`, `create_optimal_vector_index`                                 | Done   |
| Maintenance    | `ops/maintenance.rs`                    | `auto_index_if_needed`, `auto_index_if_needed_with_thresholds`                     | Done   |
| Observability  | `ops/observability.rs`                  | `analyze_table_health`, `get_query_metrics`, `get_index_cache_stats`               | Done   |
| Agentic Search | `ops/agentic.rs`                        | `agentic_search`, `QueryIntent`, `AgenticSearchConfig`                             | Done   |
| Partitioning   | `ops/partitioning.rs`                   | `suggest_partition_column`                                                         | Done   |
| Writer         | `ops/writer_impl.rs`                    | `add_documents`, `add_documents_partitioned`, `merge_insert`                       | Done   |
| Admin          | `ops/admin_impl.rs`                     | `count`, `drop_table`, `add_columns`, `alter_columns`, `create_index` (vector+FTS) | Done   |
| Checkpoint     | `checkpoint/store.rs`                   | Time-series data storage                                                           | Done   |
| Keyword        | `keyword/index.rs`, `keyword/fusion.rs` | BM25 full-text, RRF fusion, entity-aware                                           | Done   |

### 1.2 Python Bridge (RustVectorStore / PyVectorStore)

| Feature                   | Location                                          | Status |
| ------------------------- | ------------------------------------------------- | ------ |
| RustVectorStore           | `foundation/bridge/rust_vector.py`                | Done   |
| agentic_search            | PyVectorStore + RustVectorStore delegation        | Done   |
| add_documents_partitioned | PyVectorStore + RustVectorStore                   | Done   |
| analyze_table_health      | store.rs → RustVectorStore                        | Done   |
| compact                   | store.rs → RustVectorStore                        | Done   |
| get_query_metrics         | store.rs → RustVectorStore                        | Done   |
| get_index_cache_stats     | store.rs → RustVectorStore                        | Done   |
| create_index(table_name)  | search_ops.rs; creates vector + FTS default index | Done   |

### 1.3 CLI (omni db)

This project uses **`omni db`** as the vector-store operations entry (no separate `lance-cli`):

| Command                         | Description                                            | Status |
| ------------------------------- | ------------------------------------------------------ | ------ |
| `omni db health [db]`           | Table health (fragmentation, indices, recommendations) | Done   |
| `omni db compact <db>`          | Compact table                                          | Done   |
| `omni db index-stats <table>`   | Index cache stats                                      | Done   |
| `omni db query-metrics <table>` | Query metrics (in-process from agentic_search)         | Done   |
| `omni db stats`                 | Database-level stats                                   | Done   |

### 1.4 Tests

| Test                                           | Coverage                    | Status |
| ---------------------------------------------- | --------------------------- | ------ |
| test_scalar_index.rs                           | BTree/Bitmap/optimal scalar | Done   |
| test_vector_index.rs                           | HNSW/optimal vector         | Done   |
| test_maintenance.rs                            | auto_index, compact         | Done   |
| test_observability.rs                          | health                      | Done   |
| test_partitioning.rs                           | suggest_partition_column    | Done   |
| test_hybrid_search.rs                          | Hybrid search               | Done   |
| test_fusion.rs, test_entity_aware_benchmark.rs | RRF, entity-aware           | Done   |

---

## 2. Gaps / Exposure Status

### 2.1 Python Exposure (Completed)

The following APIs are now exposed on **PyVectorStore** and **RustVectorStore** (P0 done):

| Rust function                      | Rust location       | Python exposure                             |
| ---------------------------------- | ------------------- | ------------------------------------------- |
| create_btree_index(table, column)  | ops/scalar.rs       | RustVectorStore.create_btree_index          |
| create_bitmap_index(table, column) | ops/scalar.rs       | RustVectorStore.create_bitmap_index         |
| create_hnsw_index(table)           | ops/vector_index.rs | RustVectorStore.create_hnsw_index           |
| create_optimal_vector_index(table) | ops/vector_index.rs | RustVectorStore.create_optimal_vector_index |
| suggest_partition_column(table)    | ops/partitioning.rs | RustVectorStore.suggest_partition_column    |
| auto_index_if_needed(table)        | ops/maintenance.rs  | RustVectorStore.auto_index_if_needed        |

Note: Python also has `create_index(table_name)` for the admin one-shot vector + FTS creation; the above are granular index APIs.

### 2.2 CLI Extension (P1 Done)

Index creation by type under **`omni db`** is implemented:

| Feature                     | Command                                                     | Status |
| --------------------------- | ----------------------------------------------------------- | ------ |
| Create BTree index          | `omni db index create --table T --type btree --column COL`  | Done   |
| Create Bitmap index         | `omni db index create --table T --type bitmap --column COL` | Done   |
| Create HNSW index           | `omni db index create --table T --type hnsw`                | Done   |
| Create optimal-vector index | `omni db index create --table T --type optimal-vector`      | Done   |
| Partition suggestion        | `omni db partition-suggest <table>`                         | Done   |
| Table health / compact      | `omni db health` / `omni db compact`                        | Done   |

(Report “lance-cli” corresponds to **omni db** in this repo; a separate binary can be planned if needed.)

---

## 3. Suggested Next Steps

### Completed (P0): Expose Python API

- **PyO3**: `packages/rust/bindings/python/src/vector/store.rs` has the six `store_*` wrappers; PyVectorStore methods in `mod.rs`.
- **RustVectorStore**: `foundation/bridge/rust_vector.py` has the six methods delegating to `_inner` and parsing JSON/Option.
- **Tests**: `test_index_and_maintenance_api_delegate_and_parse` in `test_rust_vector_bridge_schema.py` covers them.

### Mid-term: CLI and Agentic

- **CLI**: Add `omni db index create --table T --type btree|hnsw [--column COL]` (call Python API or Rust).
- **Agentic Search**: Intent classification (e.g. LLM), configurable weights/strategy (see backlog and router docs).

---

## 4. Verification Commands

```bash
# Rust unit tests
cargo test -p omni-vector test_scalar_index
cargo test -p omni-vector test_vector_index
cargo test -p omni-vector test_maintenance
cargo test -p omni-vector test_partitioning

# Python bridge and CLI
uv run pytest packages/python/foundation/tests/ -q -k "vector or rust_vector"
uv run omni db health
uv run omni db compact skills
```

---

## 6. Future Work / Roadmap

| Item                                | Description                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **RRF / distance kernels**          | **`keyword/fusion/kernels.rs`**: Scalar `rrf_term(k, rank)`, batch `rrf_term_batch(ranks, k)`, and `distance_to_score(distance)`. All fusion paths use `rrf_term`. Batch kernel is array-in/array-out for future SIMD or Arrow compute swap-in.                                                                                                                                                                                                       |
| **Arrow lowercase kernel**          | Fusion batch lowercase is centralized in `keyword/fusion/match_util.rs` → **`lowercase_string_array(&StringArray) -> StringArray`**. That function is the single swap-in point: when Arrow provides a `compute::lowercase` (or `arrow_string::lowercase`) kernel, replace its body with a call to the kernel for SIMD-accelerated batch lowercase. Current implementation uses Rust `str::to_lowercase()` per element with Arrow-native input/output. |
| **Distributed index (>100K scale)** | Single-node LanceDB is sufficient up to roughly 100K vectors. Beyond that, consider sharding (e.g. by `skill_name`/partition), LanceDB distributed mode, or external orchestration for multi-node index build and query.                                                                                                                                                                                                                              |

---

## 5. Related Docs

- [LanceDB Version and Roadmap](lancedb-version-and-roadmap.md)
- [Omni-Vector Audit and Next Steps](omni-vector-audit-and-next-steps.md) — Audit summary, priorities, LanceDB 2.x alignment
- [Search Systems](search-systems.md)
- [Backlog](../backlog.md)
