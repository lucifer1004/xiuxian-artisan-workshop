---
type: knowledge
title: "LLM Routing Guide"
category: "llm"
tags:
  - llm
  - routing
saliency_base: 6.6
decay_rate: 0.04
metadata:
  title: "LLM Routing Guide"
---

# LLM Routing Guide

> **Status**: Active | **Version**: v1.0 | **Date**: 2026-01-16

## Overview

This guide explains how the routing system works and how LLMs can work effectively with it.

## How Routing Works

### 1. Semantic Router

The system uses semantic search to route requests to appropriate skills:

```
User Query → Semantic Router → Best Matching Skill
```

### 2. Confidence Scoring

Each routing decision has a confidence score:

| Score Range (Default) | Meaning           | Action                |
| --------------------- | ----------------- | --------------------- |
| **>= 0.75**           | High confidence   | Direct tool dispatch  |
| **0.5 - < 0.75**      | Medium confidence | Proceed with caution  |
| **< 0.5**             | Low confidence    | Ask for clarification |

Confidence mapping is configurable in settings (system: packages/conf/settings.yaml, user: $PRJ_CONFIG_HOME/xiuxian-artisan-workshop/settings.yaml; resolved from `--conf <dir>`):

```yaml
router:
  search:
    active_profile: "balanced"
    rerank: true
    profiles:
      balanced:
        high_threshold: 0.75
        medium_threshold: 0.5
        high_base: 0.90
        high_scale: 0.05
        high_cap: 0.99
        medium_base: 0.60
        medium_scale: 0.30
        medium_cap: 0.89
        low_floor: 0.10
    adaptive_threshold_step: 0.15
    adaptive_max_attempts: 3
```

### 3. Routing Factors

The router considers:

- **Vector similarity** - Semantic match to skill descriptions
- **Keyword boost** - Direct keyword matches get priority
- **Verb priority** - Action verbs (read, write, run) boost relevant skills
- **Feedback history** - Past successful routes boost future matches

## Writing Effective Queries

### 1. Be Specific

```text
# GOOD - Specific action
tool: filesystem.read_files with {"path": "src/main.py"}

# GOOD - Specific intent
tool: git.commit with {"message": "feat: add new feature"}
```

### 2. Use Action Verbs

| Action                      | Recommended Skill                    |
| --------------------------- | ------------------------------------ |
| `read`, `view`, `open`      | `filesystem.read_files`              |
| `write`, `create`, `edit`   | `filesystem.write_file`              |
| `run`, `execute`, `command` | `terminal.run_task`                  |
| `search`, `find`, `grep`    | `advanced_tools.search_project_code` |
| `commit`, `push`, `branch`  | `git.*`                              |
| `test`, `validate`          | `testing.run_tests`                  |

### 3. Include Context

```text
# GOOD - Includes context
tool: filesystem.read_files with {"path": "src/main.py"}
# Later: tool: filesystem.search_files with {"pattern": "def main"}
```

## Omega System Routing

The system routes requests through biological functional layers:

### Cortex (Planning & Decomposition)

For complex tasks requiring planning:

```text
# Cortex handles:
# - Multi-step mission decomposition
# - Parallel task DAG generation
# - Mission state management
tool: cortex.decompose_task with {"goal": "implement OAuth2 flow"}
```

### Cerebellum (Semantic Navigation)

For understanding the codebase and environment:

```text
# Cerebellum handles:
# - AST semantic scanning
# - Knowledge RAG retrieval
# - Tool discovery
tool: cerebellum.scan_codebase with {"query": "authentication logic"}
tool: knowledge.search with {"topic": "coding standards"}
```

### Hippocampus (Memory Recall)

For learning from history:

```text
# Hippocampus handles:
# - Episodic memory recall
# - Experience-driven reasoning
tool: hippocampus.recall_experience with {"query": "fix git lock error"}
```

### Homeostasis (Isolated Execution)

For safe modification and execution:

```text
# Homeostasis handles:
# - Isolated file edits
# - Git branch management
# - Command execution audit
tool: filesystem.write_file with {"path": "src/auth.py", "content": "..."}
tool: terminal.run_command with {"command": "pytest"}
```

## Hybrid Routing

### Confidence Threshold

When confidence is below configured thresholds, the system may invoke the Planner:

```
User Query → Router → [Confidence < high threshold?]
                           ↓ Yes
                    Planner (Decompose → Task List)
                           ↓
                    Executor (Loop: Execute Task → Review → Next)
```

## Routing Best Practices

### 1. Trust the Router

The router is designed to make optimal decisions. If you're unsure which skill to use, describe your intent:

```text
# Instead of guessing, ask for suggestion
tool: skill.suggest with {"task": "I need to search for all test files"}
```

### 2. Use Skill Suggestions

When uncertain:

```text
tool: skill.suggest with {"task": "find and read configuration"}
# Returns: Suggested skill with confidence score
```

### 3. Check Available Tools

List available tools through the current CLI/runtime surfaces rather than legacy external resource registries.

### 4. Use Ghost Tools for Discovery

Ghost tools provide hints about available capabilities:

```
[GHOST] advanced_tools.search_project_code
[GHOST] code_tools.count_lines
```

## Common Routing Patterns

### Pattern 1: Simple File Operation

```
User: "Read README.md"
→ Router: filesystem.read_files (confidence: 0.95)
→ Action: tool: filesystem.read_files with {"path": "README.md"}
```

### Pattern 2: Multi-step Task

```
User: "Run tests and show results"
→ Router: terminal.run_task (confidence: 0.85)
→ Action: tool: terminal.run_task with {"command": "pytest", "args": ["-v"]}
```

### Pattern 3: Complex Task (Planner)

```
User: "Refactor the entire authentication module"
→ Router: confidence: 0.65 (below threshold)
→ Action: Invoke Planner → Decompose → Execute per task
```

### Pattern 4: Git Workflow

```
User: "Commit my changes with a message"
→ Router: git.commit (confidence: 0.92)
→ Action: tool: git.commit with {"message": "feat: add auth"}
```

## Troubleshooting

### Low Confidence Routes

If routing confidence is low:

1. **Be more specific** in your query
2. **Use skill suggestions** to find the right tool
3. **Break into smaller steps** if the task is complex

### Unexpected Routing

If routed to wrong skill:

1. **Provide more context** in your query
2. **Use explicit `tool: skill.command with {...}` format**
3. **Report feedback** to improve routing

## Related Documentation

- [LLM Brain Map](./llm-brain-map.md)
- [Memory Context](./memory-context.md)
- [Cognitive Scaffolding](../human/architecture/cognitive-scaffolding.md)
- [System Layering](../explanation/system-layering.md)
