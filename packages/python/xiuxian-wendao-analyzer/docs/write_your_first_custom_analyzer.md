---
type: knowledge
metadata:
  title: "Write Your First Custom Analyzer"
---

# Write Your First Custom Analyzer

This guide is the next step after
[First Analyzer Author Tutorial](first_analyzer_author_tutorial.md).

Use it when:

1. the built-in `score_rank` strategy is not enough
2. you still want `xiuxian-wendao-py` to own transport
3. you want your Python code to own ranking behavior

## The Smallest Honest Contract

Your analyzer object only needs one method:

```python
def analyze_rows(self, rows: list[dict[str, object]]) -> list[dict[str, object]]:
    ...
```

The returned rows should:

1. preserve the fields you care about
2. include a stable `rank` field
3. be ordered as your ranking logic intends

## Minimal Example

```python
class CustomScoreAnalyzer:
    def analyze_rows(self, rows: list[dict[str, object]]) -> list[dict[str, object]]:
        ranked = sorted(rows, key=lambda row: float(row["score"]), reverse=True)
        return [
            {
                "path": str(row["path"]),
                "score": float(row["score"]),
                "rank": index + 1,
            }
            for index, row in enumerate(ranked)
        ]
```

That is enough to plug into the high-level repo pipeline.

## Connecting It To A Real Repo Workflow

Start from the runnable example:

1. [custom_repo_analyzer_workflow.py](../examples/custom_repo_analyzer_workflow.py)

The integration shape is:

1. create `WendaoTransportClient`
2. construct your analyzer object
3. call `run_repo_analysis(...)`
4. optionally call `summarize_repo_analysis(...)`

The important boundary is:

1. `xiuxian-wendao-py` owns Arrow Flight connection details
2. your analyzer object owns ranking logic
3. `xiuxian-wendao-analyzer` owns the workflow seam between them

## Recommended First Iteration

For the first custom analyzer, keep it narrow:

1. rank with fields already present in repo-search rows
2. avoid external model calls
3. keep output rows small and deterministic
4. verify behavior first against the runnable example path

## When To Stay Local Instead

If your input data is already rerank-shaped and local, prefer the local rerank
workflow instead of forcing a host-backed path.

Use:

1. `run_rerank_analysis(...)`
2. `summarize_rerank_analysis(...)`

## Related Artifacts

1. onboarding tutorial: [first_analyzer_author_tutorial.md](first_analyzer_author_tutorial.md)
2. custom analyzer example: [custom_repo_analyzer_workflow.py](../examples/custom_repo_analyzer_workflow.py)
3. package overview: [README.md](../README.md)
