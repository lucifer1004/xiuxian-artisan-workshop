---
type: knowledge
title: "LLM Guide: Writing Native Workflows"
category: "llm"
tags:
  - llm
  - workflow_runtime
saliency_base: 6.6
decay_rate: 0.04
metadata:
  title: "LLM Guide: Writing Native Workflows"
---

# LLM Guide: Writing Native Workflows

> Xiuxian Runtime Layering - Workflow Authoring Guide

## Overview

This guide defines the current standard for implementing native workflows.

All workflows should:

1. Use `NativeStateGraph` for graph construction.
2. Compile through `xiuxian_tracer.pipeline_checkpoint.compile_workflow`.
3. Persist final state through `xiuxian_foundation.workflow_state` helpers.
4. Use `get_logger()` for observability.
5. Export public entrypoints as retained runtime command functions.

## Runtime Persistence Model

Workflow state persistence is file-based.

Default storage root:

- `$PRJ_RUNTIME_DIR/xiuxian_qianji/workflow_state/<workflow_type>/`

This replaces the removed legacy LanceDB checkpoint backend.

## Standard Pattern

```python
"""skill/scripts/workflow.py - Native workflow template."""

from typing import TypedDict

from xiuxian_foundation.config.logging import get_logger
from xiuxian_foundation.workflow_state import save_workflow_state
from xiuxian_tracer.workflow_engine import END_NODE, NativeStateGraph
from xiuxian_tracer.pipeline_checkpoint import compile_workflow

logger = get_logger("skill.workflow")
_WORKFLOW_TYPE = "my_workflow"


class MyWorkflowState(TypedDict):
    request: str
    result: str
    steps: int
    error: str | None


async def node_process(state: MyWorkflowState) -> dict:
    try:
        return {
            "result": f"processed: {state['request']}",
            "steps": state["steps"] + 1,
            "error": None,
        }
    except Exception as exc:
        return {"error": str(exc), "steps": state["steps"] + 1}


def _build_workflow() -> NativeStateGraph:
    workflow = NativeStateGraph(MyWorkflowState)
    workflow.add_node("process", node_process)
    workflow.set_entry_point("process")
    workflow.add_edge("process", END_NODE)
    return workflow


# Set use_memory_saver=True when you need resumable intermediate state.
_app = compile_workflow(_build_workflow(), use_memory_saver=True)


async def my_workflow(request: str = "") -> str:
    initial_state = MyWorkflowState(
        request=request,
        result="",
        steps=0,
        error=None,
    )

    config = {"configurable": {"thread_id": f"workflow-{hash(request) % 10000}"}}
    result = await _app.ainvoke(initial_state, config=config)

    # Persist final state for audit/debug/replay.
    save_workflow_state(_WORKFLOW_TYPE, config["configurable"]["thread_id"], dict(result))

    if result.get("error"):
        return f"Error: {result['error']}"
    return result.get("result", "Done")
```

Register the callable through the retained runtime command-registration layer in
the package that exposes this workflow externally.

## Checklist

Before shipping a workflow:

1. Graph compiles at module load.
2. Entry command validates input and handles failures.
3. `thread_id` is deterministic and scoped to your session semantics.
4. Final state is persisted with `save_workflow_state(...)`.
5. Unit tests cover happy path and failure path.

Registration note:

- keep workflow construction and workflow registration as separate concerns
- expose only the final callable through the retained runtime boundary

## Anti-Patterns

Do not:

- Re-introduce LanceDB checkpoint store code in workflow paths.
- Build custom ad-hoc persistence files outside `workflow_state` helpers.
- Compile graphs inside every command invocation.
- Depend on removed checkpoint compatibility modules.

## Related Modules

- `packages/python/foundation/src/xiuxian_foundation/workflow_state.py`
- `packages/python/foundation/src/xiuxian_tracer/pipeline_checkpoint.py`
- `packages/python/foundation/src/xiuxian_tracer/workflow_engine.py`
