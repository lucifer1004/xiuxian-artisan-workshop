---
type: knowledge
metadata:
  title: "First Analyzer Author Tutorial"
---

# First Analyzer Author Tutorial

This tutorial is the shortest honest path for a new Python analyzer author.

`xiuxian-wendao-analyzer` sits above `xiuxian-wendao-py`:

1. `xiuxian-wendao-py` owns Arrow Flight transport and typed Wendao contracts
2. `xiuxian-wendao-analyzer` owns Python-side analyzer workflows and ranking logic

Use this tutorial by workflow.

## Workflow 1: Local Rerank Experiment

Use this when you already have rerank-shaped rows in Python and want to test
local ranking logic without a live host.

Run:

```bash
uv run python examples/local_rerank_workflow.py
```

This path exercises:

1. `run_rerank_analysis(...)`
2. `summarize_rerank_analysis(...)`
3. the default `linear_blend` analyzer

This is the fastest path for scoring experiments.

## Workflow 2: Host-Backed Repo Search With Built-In Ranking

Use this when you want real Wendao repo-search data and the built-in
`score_rank` strategy is sufficient.

Run:

```bash
uv run python examples/repo_search_workflow.py --host 127.0.0.1 --port 8815 --query-text alpha --path-prefix src/
```

This path exercises:

1. `run_repo_analysis(...)`
2. `summarize_repo_analysis(...)`
3. a real `wendao_search_flight_server`
4. built-in `AnalyzerConfig(strategy="score_rank")`

Choose this path first if you need real search data but do not yet need custom
Python ranking logic.

## Workflow 3: Host-Backed Repo Search With A Custom Python Analyzer

Use this when you want real Wendao repo-search data but also need your own
Python ranking behavior.

Start from:

```bash
uv run python examples/custom_repo_analyzer_workflow.py --help
```

The example shows the intended boundary:

1. `xiuxian-wendao-py` still owns transport
2. your analyzer object owns Python ranking logic
3. `run_repo_analysis(...)` remains the high-level pipeline entrypoint

The custom analyzer contract is intentionally small:

1. accept `list[dict[str, object]]`
2. return ranked `list[dict[str, object]]`
3. include a stable `rank` field in the returned rows

## When To Drop Lower

Use lower-level helpers only when the high-level workflow objects are too much:

1. choose `analyze_*` helpers when you want analyzed rows only
2. choose `run_*` helpers when you want both the input artifact and typed output
3. choose `summarize_*` helpers when you only need the top-hit snapshot

For most new users, start with:

1. `run_repo_analysis(...)` for host-backed repo workflows
2. `run_rerank_analysis(...)` for local rerank workflows

## Current Boundary

This tutorial intentionally does not claim more than the package currently
proves:

1. repo-search is the real-host workflow
2. local rerank is the analyzer-owned local workflow
3. there is not yet a live host claim for analyzer-shaped rerank input rows

## Related Artifacts

1. package overview: [README.md](../README.md)
2. local rerank example: [local_rerank_workflow.py](../examples/local_rerank_workflow.py)
3. host-backed repo example: [repo_search_workflow.py](../examples/repo_search_workflow.py)
4. custom analyzer example: [custom_repo_analyzer_workflow.py](../examples/custom_repo_analyzer_workflow.py)
