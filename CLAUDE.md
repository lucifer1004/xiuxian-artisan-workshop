---
type: knowledge
metadata:
  title: "CyberXiuXian Artisan Workshop (赛博修仙创意工坊)"
---

# CyberXiuXian Artisan Workshop (赛博修仙创意工坊)

(赛博修仙创意工坊)

> One Tool + Trinity Architecture
> Single Entry Point: `@omni("skill.command")`

Quick Reference: `docs/explanation/trinity-architecture.md` | `docs/skills.md`

---

## MANDATORY READING

**All LLMs MUST read these documents BEFORE making any changes:**

### 1. Engineering Protocol (Python/Rust)

**File: `docs/reference/odf-ep-protocol.md`**

Universal engineering standards:

- Code Style: Type hints, async-first, Google docstrings
- Naming Conventions: snake_case, PascalCase, UPPER_SNAKE_CASE
- Module Design: Single responsibility, import rules, dependency flow
- Error Handling: Fail fast, rich context
- Testing Standards: Unit tests required, parametrized tests
- Git Workflow: Commit format, branch naming

### 2. Project Execution Standard

**File: `docs/reference/project-execution-standard.md`**

Project-specific implementations:

- Rust/Python cross-language workflow
- Project namespace conventions and examples
- SSOT utilities: `SKILLS_DIR()`, `PRJ_DATA()`, `get_setting()`
- Build and test commands

### 3. RAG/Representation Protocol

**File: `docs/reference/odf-rep-protocol.md`**

Memory system, knowledge indexing, context optimization

---

## Critical Rules

### Cognitive Alignment & Protocols

- **Protocol Adherence**: Strictly follow the instructions in each skill's `SKILL.md`.
- **Re-anchoring**: If you drift from the protocol or attempt unauthorized tool calls, the Gatekeeper will inject the correct `SKILL.md` rules into your context to force re-alignment.
- **Overload Management**: Avoid activating more than 5 skills simultaneously. If you see a `COGNITIVE LOAD WARNING`, disable unused skills to maintain precision.
- **Tool Selection**: Prioritize skill-specific MCP tools over generic shell commands for all write operations.

### No Global Lint Suppressions in Rust

**ABSOLUTE PROHIBITION**: You are STRICTLY FORBIDDEN from inserting `#![allow(missing_docs, unused_imports, dead_code)]` or any other `#![allow(...)]` attributes at the file or module level in Rust code. Doing so destroys modern engineering standards and bypasses essential checks. You MUST fix the underlying code issues (write the docs, remove the imports, delete dead code) instead of silencing the compiler.

### Language

**All project content in English**: All documentation, commit messages, and committed content in this repository must be written in English (`docs/`, skill docs, code comments, commit messages). This is a persistent rule; do not add or commit non-English docs or messages.

### Git Commit

**Use `/commit` slash command** - Never `git commit` via terminal.

### Rust/Python Cross-Language Development

> **Read First**: `docs/reference/project-execution-standard.md`

Follow the **strict workflow**:

```
Rust Implementation → Add Rust Test → cargo nextest run PASSED
                 ↓
Python Integration → Add Python Test → pytest PASSED
                 ↓
Build & Verify → Full integration test
```

## Incremental Evolution Protocol

To prevent context bloating and "hallucination spirals," follow the **Fragmented Planning Model**:

1. **[TASK-LOCAL-RESEARCH]**: Each sub-task MUST have its own independent [Research] phase. Never search/read for Task N+1 until Task N is `[DONE]`.
2. **[PHYSICAL-SYNC-GATE]**: Verify "physical reality" via `ls` or `cat` on the target path before ANY implementation.
3. **[CHECKPOINT-SIGN-OFF]**: After each atomic code change, run relevant unit tests. Only ask for a "Pulse Check" after tests pass.

## ExecPlans & Holistic Evolution

When writing complex features or refactors, use an ExecPlan (`.agent/PLANS.md`).

### Blueprint Adherence

Tasks under an active strategic blueprint MUST strictly adhere to its architectural mandates.

- **Active Reference**: `[[.data/blueprints/project_anchor_semantic_addressing.md]]` (Project AnchoR: Wendao Semantic Addressing Kernel).

### Holistic Evolution Workflow (Triple-Sync Protocol)

1. **Blueprint Check**: Check if task falls under a strategic blueprint.
2. **GTD Synchronization**: Update today's GTD (`docs/GTD/DAILY_YYYY_MM_DD.md`).
3. **ExecPlan Creation**: Create a formal plan (`.cache/codex/execplans/<slug>.md`) referencing the blueprint.
4. **Implementation**: Execute per the plan.

## Project Structure & Sovereignty

- `packages/rust/crates/*`: **Sovereign Kernel**. Core logic, memory systems, indexing.
- `packages/rust/bindings/python`: PyO3 bridge (`xiuxian-core-rs`).
- `packages/python/*`: **Utility Adapters**. Lightweight glue only.
- `internal_skills/`: **Kernel-Level Siddhis (本命神通)** bound to Rust logic.

## Environment & Directory Layout

Use these directories for all project-local paths. Do not hardcode paths.

| Env Var           | Default | Purpose                        |
| ----------------- | ------- | ------------------------------ |
| `PRJ_ROOT`        | (root)  | Project root.                  |
| `PRJ_DATA_HOME`   | `.data` | Persistent project data.       |
| `PRJ_RUNTIME_DIR` | `.run`  | Runtime state (logs, sockets). |

---

## Essential Commands

- `just validate` - fmt, lint, test
- `uv run pytest` - Run Python tests
- `/mcp enable orchestrator` - Reconnect omni mcp

---

## Directory Structure

```
.claude/commands/     # Slash command templates
assets/skills/*/       # Skill implementations (scanned recursively in scripts/)
docs/                 # Documentation (see docs/skills.md for index)
.cache/               # Repomix skill contexts (auto-generated)
```
