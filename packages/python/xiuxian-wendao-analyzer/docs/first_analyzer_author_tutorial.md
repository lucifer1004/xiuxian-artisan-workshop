# First Analyzer Author Tutorial

`xiuxian-wendao-analyzer` sits above the Rust-owned Wendao transport stack:

1. `wendao-core-lib` owns Flight transport and typed contracts
2. `wendao-arrow-interface` owns downstream session ergonomics
3. `xiuxian-wendao-analyzer` owns analysis over rows and tables that already
   came back from Rust

The important boundary is simple: this package does not own rerank workflows.
If Rust returns a table, this package can analyze that table. It does not
define a separate Python-side rerank runtime.

## Workflow 1: Offline Repo Search Authoring With Scripted Results

Start here when you want the fastest local loop and do not need a live Flight
host.

```bash
uv run python examples/scripted_repo_search_workflow.py
```

That workflow uses:

1. `WendaoArrowSession.for_repo_search_testing(...)`
2. `run_repo_analysis(...)`
3. `summarize_repo_analysis(...)`

## Workflow 2: Host-Backed Repo Search With Built-In Ranking

Use this when you want real Wendao repo-search data and the built-in
`score_rank` analyzer is enough.

```bash
uv run python examples/repo_search_workflow.py --help
```

The built-in path is:

1. `run_repo_analysis(...)`
2. `summarize_repo_analysis(...)`
3. `AnalyzerConfig(strategy="score_rank")`

## Workflow 3: Host-Backed Repo Search With A Custom Python Analyzer

Use this when Rust should fetch the rows but your ranking logic is custom.

```bash
uv run python examples/custom_repo_analyzer_workflow.py --help
```

That workflow keeps ownership clean:

1. Rust fetches the data
2. your analyzer object implements `analyze_rows(...)`
3. `run_repo_analysis(...)` applies it to the returned rows

## Workflow 4: PDF Attachment Search Then Analyze The Returned Table

Use this when Rust should query `/search/attachments` and your Python analyzer
should only work over the returned PDF rows.

```bash
uv run python examples/attachment_pdf_analyzer_workflow.py
```

That workflow keeps the boundary explicit:

1. `attachment_search_request(...)` builds the Rust-owned query contract
2. `WendaoArrowSession.attachment_search(...)` fetches the Arrow table
3. `run_table_analysis(...)` analyzes the returned table in Python

If you already have a live Flight endpoint that serves `/search/attachments`,
switch the same example to endpoint mode with `--mode endpoint --port <port>`.

## Workflow 5: Analyze An Already Materialized Rust Query Result

If another package already fetched the data, analyze it directly.

```python
from wendao_arrow_interface import WendaoArrowSession
from xiuxian_wendao_analyzer import analyze_table

session = WendaoArrowSession.for_repo_search_testing(
    [{"path": "src/lib.rs", "score": 0.9}]
)
result = session.repo_search("alpha", limit=1)
ranked = analyze_table(result.table)
```

The same pattern applies to any Rust-owned route. For example, if you later
fetch `/rerank/flight` through `wendao-arrow-interface`, hand the returned table
to `analyze_table(...)` instead of looking for a rerank-specific analyzer API.
