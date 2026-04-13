---
type: knowledge
metadata:
  title: "xiuxian-wendao-analyzer"
---

# xiuxian-wendao-analyzer

`xiuxian-wendao-analyzer` is a beta Python analyzer-layer package built on top
of `wendao-core-lib`.

Its boundary is intentionally narrow:

1. analyze rows and Arrow tables that already came back from Rust-owned Wendao
   query or exchange surfaces
2. provide one built-in deterministic ranking strategy for score-carrying rows
3. expose lightweight run objects and summary models for downstream callers
4. reuse `wendao-arrow-interface` for offline scripted authoring and testing

It does not own:

1. rerank transport contracts
2. local rerank semantics
3. Flight metadata assembly
4. any Python-hosted shadow of Rust runtime logic

## Current Beta Surface

The current beta exports:

1. `AnalyzerConfig`
2. `ScoreRankAnalyzer`
3. `AnalyzerResultRow`
4. `AnalysisSummary`
5. `RowsAnalysisRun`
6. `TableAnalysisRun`
7. `QueryAnalysisRun`
8. `RepoAnalysisRun`
9. `build_analyzer(config)`
10. `analyze_rows(...)`
11. `analyze_table(...)`
12. `analyze_query(...)`
13. `analyze_repo_search(...)`
14. `analyze_repo_query_text(...)`
15. `run_rows_analysis(...)`
16. `run_table_analysis(...)`
17. `run_query_analysis(...)`
18. `run_repo_analysis(...)`
19. `run_repo_search_analysis(...)`
20. summary helpers over the same rows, table, query, and repo-search runs

The built-in strategy is intentionally small:

1. `score_rank`
2. consumes rows that already carry a numeric `score`
3. emits the same rows with a stable integer `rank`

## Boundary Reading

The package split is:

1. `wendao-core-lib` owns Arrow Flight transport and typed contracts
2. `wendao-arrow-interface` owns downstream session ergonomics and scripted
   fixtures
3. `xiuxian-wendao-analyzer` owns analysis over already materialized results

That means rerank stays transport-owned. If you need to analyze a rerank result,
fetch it through `wendao-core-lib` or `wendao-arrow-interface`, then hand the
returned rows or table into `analyze_rows(...)` or `analyze_table(...)`.

## Workflow Selection Guide

| Workflow                                              | Recommended entrypoint                                                                                     | Analyzer ownership                            | Host involvement                       | Validation status |
| ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | --------------------------------------------- | -------------------------------------- | ----------------- |
| Offline repo-search authoring with scripted results   | `WendaoArrowSession.for_repo_search_testing(...)` + `run_repo_analysis(session.client, ...)`               | downstream user code                          | none                                   | local covered     |
| PDF attachment search then analyze the returned table | `attachment_search_request(...)` + `WendaoArrowSession.attachment_search(...)` + `run_table_analysis(...)` | downstream user code                          | scripted by default, endpoint optional | local covered     |
| Repo search with built-in ranking                     | `run_repo_analysis(...)` + `summarize_repo_analysis(...)`                                                  | built-in `score_rank`                         | real `wendao_search_flight_server`     | real-host covered |
| Repo search with a custom Python analyzer             | `run_repo_analysis(...)` + `summarize_repo_analysis(...)` + `analyzer=<your analyzer object>`              | downstream user code                          | real `wendao_search_flight_server`     | real-host covered |
| Analyze an already materialized Rust query result     | `analyze_rows(...)` or `analyze_table(...)`                                                                | built-in `score_rank` or downstream user code | depends on who fetched the data        | local covered     |

## Documentation Set

This README is the intended v1 public docs surface index:

1. [`docs/first_analyzer_author_tutorial.md`](docs/first_analyzer_author_tutorial.md)
2. [`docs/write_your_first_custom_analyzer.md`](docs/write_your_first_custom_analyzer.md)
3. [`docs/release_and_compatibility_policy.md`](docs/release_and_compatibility_policy.md)
4. [`docs/external_consumer_checklist.md`](docs/external_consumer_checklist.md)

## Examples

The shipped example set is now:

1. [`examples/scripted_repo_search_workflow.py`](examples/scripted_repo_search_workflow.py)
   - offline analyzer authoring with `WendaoArrowSession.for_repo_search_testing(...)`
2. [`examples/attachment_pdf_analyzer_workflow.py`](examples/attachment_pdf_analyzer_workflow.py)
   - scripted-by-default PDF attachment search over Rust-returned rows, with optional endpoint mode
3. [`examples/repo_search_workflow.py`](examples/repo_search_workflow.py)
   - host-backed repo-search analysis with built-in `score_rank`
4. [`examples/custom_repo_analyzer_workflow.py`](examples/custom_repo_analyzer_workflow.py)
   - host-backed repo-search analysis with a custom analyzer object
5. [`examples/host_backed_repo_search_beta_smoke.py`](examples/host_backed_repo_search_beta_smoke.py)
   - one-shot beta smoke for the full host-backed repo-search path

Example commands:

```bash
uv run python examples/scripted_repo_search_workflow.py
uv run python examples/attachment_pdf_analyzer_workflow.py
uv run python examples/repo_search_workflow.py --help
uv run python examples/custom_repo_analyzer_workflow.py --help
uv run python examples/host_backed_repo_search_beta_smoke.py --mode custom --port 0
```

## Beta Readiness

ready now:

1. offline scripted repo-search authoring
2. scripted PDF attachment analysis over Rust-returned attachment-search tables
3. host-backed repo-search analysis with built-in or custom analyzers
4. generic rows/table/query analysis over Rust-returned data

known gaps before broader adoption:

1. no GA-level release promise yet
2. no analyzer-owned rerank helper surface
3. callers still need `uv run python ...` from the package directory for the
   shipped examples

## Beta Freeze Audit

The current package boundary is now intentionally lockable as `0.2.1`.

frozen for this beta trial:

1. the five shipped examples above
2. `WendaoArrowSession.for_repo_search_testing(...)` as the documented offline
   author workflow
3. `WendaoArrowSession.attachment_search(...)` plus `run_table_analysis(...)`
   as the documented PDF attachment workflow seam
4. `run_repo_analysis(...)` and `run_query_analysis(...)` as the host-backed
   analyzer entrypoints
5. the rule that analyzer-owned rerank helpers are out of scope

not frozen for this beta trial:

1. helper symmetry for every possible Wendao route
2. future analyzer strategies beyond `score_rank`
3. additional convenience wrappers over already materialized transport results

current freeze rule:

1. workflow-frozen, not helper-frozen
2. transport-owned rerank remains in `wendao-core-lib` and
   `wendao-arrow-interface`

## Beta Exit Audit

exit-ready now:

1. real-host repo-search coverage exists through `wendao_search_flight_server`
2. offline authoring is available through the scripted session surface
3. docs and examples align with the Rust-query-first analyzer boundary

not exit-ready yet:

1. no GA-level versioning promise yet
2. no broader downstream feedback cycle yet
3. no committed compatibility window beyond this beta baseline
