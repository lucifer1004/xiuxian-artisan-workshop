---
type: knowledge
metadata:
  title: "xiuxian-wendao-analyzer"
---

# xiuxian-wendao-analyzer

`xiuxian-wendao-analyzer` is a beta Python analyzer-layer package built on top
of `xiuxian-wendao-py`.

Current scope for this beta slice:

1. provide an importable package boundary
2. depend explicitly on `xiuxian-wendao-py`
3. reserve the future home for Python-local analyzer strategies and runtime
   helpers

The current milestone now also includes a minimal analyzer surface:

1. `AnalyzerConfig`
2. `AnalyzerResultRow`
3. `AnalysisSummary`
4. `QueryAnalysisRun`
5. `RowsAnalysisRun`
6. `TableAnalysisRun`
7. `build_analyzer(config)`
8. `analyze_rows(...)`
9. `analyze_table(...)`
10. `analyze_query(...)`
11. `analyze_repo_search(...)`
12. `analyze_repo_query_text(...)`
13. `analyze_rerank_rows(...)`
14. `analyze_rerank_table(...)`
15. `run_rows_analysis(...)`
16. `run_table_analysis(...)`
17. `run_query_analysis(...)`
18. `run_repo_analysis(...)`
19. `run_repo_search_analysis(...)`
20. `run_rerank_analysis(...)`
21. `run_rerank_table_analysis(...)`
22. summary helpers over generic query, repo, and rerank analysis artifacts
23. direct summary helpers for typed repo-search requests, repo query text,
    local rerank rows, and local rerank tables
24. direct summary helpers for generic host-backed Flight queries
25. direct summary helpers for local rows and local Arrow tables
26. run-object summary helpers for local row and table pipelines
27. typed-result summary helpers on top of the same substrate and analyzer APIs
28. typed `*_results(...)` helpers on top of the same raw APIs

The first strategy is intentionally narrow and deterministic:

1. linear blend only
2. consumes rerank-shaped rows with:
   - `doc_id`
   - `vector_score`
   - `embedding`
   - `query_embedding`
3. emits ranked rows with:
   - `doc_id`
   - `vector_score`
   - `semantic_score`
   - `final_score`
   - `rank`

The package now also includes one built-in repo-search oriented strategy:

1. `score_rank`
2. consumes rows that already carry a numeric `score`
3. re-ranks them deterministically by descending score

The current transport-backed integration seam is intentionally narrow:

1. `analyze_query(...)` consumes `WendaoTransportClient` and
   `WendaoFlightRouteQuery`
2. it reuses `read_query_table(...)` from `xiuxian-wendao-py`
3. it does not rebuild raw Arrow/Flight metadata assembly inside this package

Current status:

1. package skeleton landed
2. deterministic baseline analyzer landed
3. transport-backed query helper landed
4. one real-host integration path landed for `analyze_query(...)` with an
   explicit repo-search analyzer
5. typed repo-search convenience helper landed on top of the same transport
   substrate
6. typed analyzer result model landed for downstream callers that do not want
   to consume raw `dict` rows
7. typed rerank convenience helper landed for local analyzer-owned rerank input
8. Arrow-native rerank table convenience helper landed on top of the same
   local analyzer seam
9. built-in `score_rank` now makes repo-search analysis usable without a
   custom analyzer class
10. high-level repo-search query-text entrypoint now exists on top of the same
    host-backed repo-search seam
11. analyzer-owned repo-search pipeline result now returns both the effective
    request and typed analyzed rows
12. analyzer-owned local rerank pipeline result now returns both input rows and
    typed analyzed rows
13. analyzer-owned summaries now give both pipeline families a lightweight
    top-row snapshot surface
14. direct summary entrypoints now exist for host-backed typed repo-search
    requests, repo query text, local rerank rows, and local rerank tables
15. typed-request repo-search pipeline entrypoints now exist alongside the
    query-text pipeline entrypoints
16. Arrow-native rerank callers now also have a pipeline-plus-summary surface
    instead of dropping back to lower-level table helpers
17. generic host-backed query callers now also have a typed pipeline-plus-summary
    surface instead of stopping at `analyze_query(...)`
18. generic host-backed query callers now also have a one-shot summary entrypoint
    instead of manually chaining pipeline and summary helpers
19. local row and Arrow-table callers now also have one-shot summary helpers
    instead of manually chaining analyze-plus-parse-plus-summary helpers
20. local row and Arrow-table callers now also have typed pipeline run objects
    instead of stopping at raw or typed result helpers
21. those same local run objects now also have direct summary helpers
22. typed-result callers now also have direct summary helpers instead of always
    dropping back to `summarize_result_rows(...)`
23. host-backed typed-result summary helpers now also have real-host coverage on
    the repo-search path
24. package-root exports now have a dedicated regression guard
25. package-root exports now also lock expected symbol kinds so public classes
    and helper callables do not silently drift
26. default rerank-shaped baseline analyzer still remains local-only until a
    live host exposes analyzer-shaped input rows directly

## Locked Beta Baseline

The current package boundary is now intentionally lockable as `0.1.1`.

This locked beta baseline means:

1. the package root now exports a concrete `__version__`
2. the documented workflow set remains:
   - host-backed repo-search analysis
   - host-backed rerank exchange analysis
   - local rerank analysis
3. the current beta validation expectation is Flight-first:
   - repo-search flows follow the `schema_version=v2` contract
   - rerank exchange flows continue to rely on the live `/rerank/flight` seam
4. future work may extend helpers, but should not silently break these
   documented workflows without an explicit migration note

This package is intended to mirror the role split already present in Julia:

1. `.data/WendaoArrow` owns transport and contract helpers
2. `.data/WendaoAnalyzer` owns analyzer logic

The Python analogue is:

1. `xiuxian-wendao-py` owns Arrow/Flight transport and typed contract access
2. `xiuxian-wendao-analyzer` owns future Python-local analyzer semantics

## V1 Sufficiency Baseline

The current analyzer surface is operationally sufficient for a first external
user slice.

That sufficiency claim is intentionally narrow:

1. host-backed repo-search analysis is available through generic query,
   typed-request, and query-text entrypoints
2. local rerank analysis is available through typed rows and Arrow tables
3. callers can choose between:
   - raw analyzed row dictionaries
   - typed `AnalyzerResultRow` results
   - typed run objects
   - lightweight `AnalysisSummary` views
4. the package root export surface now has both:
   - presence coverage
   - symbol-kind coverage
5. repo-search host-backed summary and typed-result paths have real-host
   coverage through `wendao_search_flight_server`

Current non-goals for this v1 boundary:

1. no live host claim for caller-supplied Python analyzer execution inside rerank
   exchange
2. no plugin migration from `xiuxian-wendao-py`
3. no package-local Flight server or transport protocol ownership
4. no further helper-symmetry expansion unless a new user workflow appears

One important boundary detail:

1. the substrate already proves a live `/rerank/flight` transport route
2. this package now exposes one host-backed rerank workflow above that transport
   seam:
   - `run_rerank_exchange_analysis(...)`
   - `summarize_rerank_exchange(...)`
3. caller-supplied Python analyzer logic over rerank-shaped data still remains
   a local workflow

The practical reading is:

1. `xiuxian-wendao-analyzer` is now enough to support early downstream analyzer
   authors on top of `xiuxian-wendao-py`
2. the next meaningful work should come from a new analyzer workflow gap, not
   from continuing to mirror every existing helper shape

Reference workflows for this baseline:

1. host-backed repo-search workflow:
   - `run_repo_analysis(...)`
   - `summarize_repo_analysis(...)`
   - built-in `AnalyzerConfig(strategy="score_rank")`
2. host-backed rerank exchange workflow:
   - `run_rerank_exchange_analysis(...)`
   - `summarize_rerank_exchange(...)`
   - runtime-owned rerank scoring from live `/rerank/flight`
3. local rerank workflow:
   - `run_rerank_analysis(...)`
   - `summarize_rerank_analysis(...)`
   - default `linear_blend` analyzer
4. custom repo analyzer workflow:
   - pass `analyzer=<your analyzer object>` into `run_repo_analysis(...)`
   - keep `xiuxian-wendao-py` responsible for Arrow/Flight transport
   - keep `xiuxian-wendao-analyzer` responsible for Python ranking logic

## Workflow Selection Guide

Use this package by workflow, not by helper symmetry.

| Workflow                                  | Recommended entrypoint                                                                        | Analyzer ownership                                 | Host involvement                   | Validation status         |
| ----------------------------------------- | --------------------------------------------------------------------------------------------- | -------------------------------------------------- | ---------------------------------- | ------------------------- |
| Repo search with built-in ranking         | `run_repo_analysis(...)` + `summarize_repo_analysis(...)`                                     | built-in `score_rank`                              | real `wendao_search_flight_server` | real-host covered         |
| Repo search with a custom Python analyzer | `run_repo_analysis(...)` + `summarize_repo_analysis(...)` + `analyzer=<your analyzer object>` | downstream user code                               | real `wendao_search_flight_server` | real-host covered         |
| Host-backed rerank exchange               | `run_rerank_exchange_analysis(...)` + `summarize_rerank_exchange(...)`                        | runtime-owned rerank scoring                       | real `wendao_flight_server`        | real-host covered         |
| Generic host-backed query analysis        | `run_query_analysis(...)` + `summarize_query(...)`                                            | caller-supplied analyzer                           | depends on selected Flight route   | repo-search route covered |
| Local rerank analysis over typed rows     | `run_rerank_analysis(...)` + `summarize_rerank_analysis(...)`                                 | default `linear_blend` or caller-supplied analyzer | none                               | local covered             |
| Local rerank analysis over Arrow tables   | `run_rerank_table_analysis(...)` + `summarize_rerank_table(...)`                              | default `linear_blend` or caller-supplied analyzer | none                               | local covered             |

Recommended reading of that matrix:

1. if you need real Wendao search data, start from `run_repo_analysis(...)`
2. if you need runtime-owned live rerank results, use
   `run_rerank_exchange_analysis(...)`
3. if you are experimenting with Python-local scoring over rerank-shaped data,
   stay on the local rerank path
4. only drop to lower-level `analyze_*` helpers when you explicitly do not want
   run objects or summaries
5. do not confuse host-backed rerank exchange with local rerank scoring; they
   are intentionally different workflows

## Runnable Examples

Example entrypoints live under `examples/`:

1. `examples/local_rerank_workflow.py`
   - fully runnable local analyzer example
   - demonstrates `run_rerank_analysis(...)` plus
     `summarize_rerank_analysis(...)`
2. `examples/repo_search_workflow.py`
   - host-backed repo-search template
   - demonstrates `run_repo_analysis(...)` plus
     `summarize_repo_analysis(...)`
   - expects an already-running Flight host
3. `examples/custom_repo_analyzer_workflow.py`
   - host-backed repo-search example with a custom Python analyzer object
   - demonstrates `run_repo_analysis(...)` plus
     `summarize_repo_analysis(...)`
   - keeps transport in `xiuxian-wendao-py` and ranking logic in user code
4. `examples/host_backed_repo_search_beta_smoke.py`
   - one-shot beta smoke for the full host-backed path
   - seeds a temporary sample repo workspace
   - boots `wendao_search_flight_server`
   - runs either `examples/repo_search_workflow.py` or
     `examples/custom_repo_analyzer_workflow.py` against that live host
5. `examples/rerank_exchange_workflow.py`
   - host-backed rerank exchange example
   - demonstrates `run_rerank_exchange_analysis(...)` plus
     `summarize_rerank_exchange(...)`
   - expects an already-running `wendao_flight_server`
6. `examples/host_backed_rerank_beta_smoke.py`
   - one-shot beta smoke for the full host-backed rerank path
   - boots `wendao_flight_server`
   - runs `examples/rerank_exchange_workflow.py` against that live host

Example commands:

```bash
uv run python examples/local_rerank_workflow.py
uv run python examples/repo_search_workflow.py --help
uv run python examples/repo_search_workflow.py --host 127.0.0.1 --port 8815 --query-text alpha --path-prefix src/
uv run python examples/custom_repo_analyzer_workflow.py --help
uv run python examples/host_backed_repo_search_beta_smoke.py --port 0
uv run python examples/host_backed_repo_search_beta_smoke.py --mode custom --port 0
uv run python examples/host_backed_repo_search_beta_smoke.py --port 0 --keep-workspace
uv run python examples/rerank_exchange_workflow.py --help
uv run python examples/rerank_exchange_workflow.py --host 127.0.0.1 --port 8816 --top-k 2
uv run python examples/host_backed_rerank_beta_smoke.py --port 0
```

During this beta, treat `uv run python ...` as the default example invocation
shape. Do not assume plain `python examples/...` will work unless the package
has already been installed into that interpreter environment.

## Tutorial

For a workflow-first onboarding path, start with
[docs/first_analyzer_author_tutorial.md](docs/first_analyzer_author_tutorial.md).

For the next step after that, use
[docs/write_your_first_custom_analyzer.md](docs/write_your_first_custom_analyzer.md).

For the current beta compatibility contract, see
[docs/release_and_compatibility_policy.md](docs/release_and_compatibility_policy.md).

For first trial setup and environment checks, use
[docs/external_consumer_checklist.md](docs/external_consumer_checklist.md).

## Documentation Set

The current v1 documentation set is intentionally small:

1. this `README.md` is the package overview and workflow index
2. `docs/first_analyzer_author_tutorial.md` is the first-stop onboarding guide
3. `docs/write_your_first_custom_analyzer.md` is the follow-up authoring guide
4. `docs/release_and_compatibility_policy.md` defines the current beta
   compatibility reading
5. `docs/external_consumer_checklist.md` is the first-trial checklist for
   external consumers

Current doc-set rule:

1. new onboarding material should extend one of these documents unless it
   represents a genuinely new workflow class
2. this package should not grow a large tree of overlapping tutorial fragments
3. the tests currently lock this documentation set as the intended v1 public
   docs surface

## Beta Readiness

Current readiness for external trial use:

1. ready now:
   - host-backed repo-search with built-in `score_rank`
   - host-backed repo-search with a custom Python analyzer
   - host-backed rerank exchange through live `/rerank/flight`
   - local rerank experimentation over typed rows and Arrow tables
   - shipped examples run through `uv run python ...` from the package directory
2. known gaps before broader adoption:
   - no live-host custom Python analyzer rerank workflow
   - no plugin migration story yet
   - no GA-level release promise yet
3. current stop condition for this beta slice:
   - do not expand the API surface by default
   - only add new package surface when a concrete external-user workflow is
     blocked by the current boundary
   - prefer hardening examples, docs, and compatibility notes over new helper
     symmetry

This beta package intentionally does not yet provide plugin migration,
live-host custom Python analyzer rerank execution, or a package-local Flight
runtime.

## Beta Exit Audit

Current beta-exit reading:

1. exit-ready now:
   - built-in host-backed repo-search workflow
   - custom-analyzer host-backed repo-search workflow
   - host-backed rerank exchange workflow
   - local rerank workflow over typed rows and Arrow tables
   - runnable examples for all four workflow paths
   - one-shot host-backed beta smoke with:
     - built-in mode
     - custom mode
     - `--keep-workspace`
2. not exit-ready yet:
   - live-host custom Python analyzer rerank workflow
   - plugin migration story
   - GA-level public API freeze for all helper-shaped symbols
3. current beta-exit gate:
   - no new helper symmetry by default
   - prefer proving one missing external workflow over adding another wrapper
   - only claim broader readiness after a new external-user trial exposes no
     blocking gap in the current documented workflows

## Beta Freeze Audit

Current beta-freeze reading:

1. frozen for this beta trial:
   - host-backed repo-search workflows
   - host-backed rerank exchange workflow
   - local rerank workflows
   - shipped example set:
     - `examples/local_rerank_workflow.py`
     - `examples/repo_search_workflow.py`
     - `examples/custom_repo_analyzer_workflow.py`
     - `examples/rerank_exchange_workflow.py`
     - `examples/host_backed_repo_search_beta_smoke.py`
     - `examples/host_backed_rerank_beta_smoke.py`
2. not frozen for this beta trial:
   - helper-shaped convenience symbols beyond the documented workflow entrypoints
   - live-host custom Python analyzer rerank workflow
   - plugin migration story
3. current freeze rule:
   - do not add another shipped workflow or example by default
   - only widen the beta surface after a new external-user trial exposes a
     missing workflow class
   - prefer changing docs, examples, or compatibility notes before adding
     another top-level wrapper
