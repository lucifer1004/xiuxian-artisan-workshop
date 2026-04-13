# External Consumer Checklist

Use this checklist before trying `xiuxian-wendao-analyzer` from another
workspace.

## Environment

Confirm:

1. Python `>=3.12`
2. `pyarrow>=14.0.0`
3. `uv` is available
4. you plan to run the shipped examples with `uv run python ...`

Plain `python examples/...` is not the recommended path for the packaged
examples because the package-local workspace wiring is already encoded in the
`uv` setup.

## Fast Local Validation

Run one shipped example unchanged:

```bash
uv run python examples/scripted_repo_search_workflow.py
uv run python examples/attachment_pdf_analyzer_workflow.py
```

That proves:

1. the package imports cleanly
2. `WendaoArrowSession.for_repo_search_testing(...)` works in your environment
3. `WendaoArrowSession.attachment_search(...)` works for a scripted PDF attachment workflow
4. the analyzer package can process scripted Rust-shaped repo-search rows

## Host-Backed Repo Search

If you want real Wendao search results, confirm these binaries exist:

```bash
cargo build -p xiuxian-wendao --features julia --bin wendao_search_flight_server --bin wendao_search_seed_sample
```

Then you can seed a temporary workspace and run:

```bash
tmp_root="$(mktemp -d)"
wendao_search_seed_sample alpha/repo "$tmp_root"
uv run python examples/repo_search_workflow.py --host 127.0.0.1 --port 8815
uv run python examples/custom_repo_analyzer_workflow.py --host 127.0.0.1 --port 8815
uv run python examples/host_backed_repo_search_beta_smoke.py --port 0
uv run python examples/host_backed_repo_search_beta_smoke.py --mode custom --port 0
uv run python examples/host_backed_repo_search_beta_smoke.py --port 0 --keep-workspace
```

Use `--keep-workspace` when you want to inspect the seeded repo after the smoke
run.

## Generic Table Or Row Analysis

If another package already fetched a Rust-owned query result, you do not need a
repo-search-specific entrypoint.

Use:

1. `analyze_rows(...)`
2. `analyze_table(...)`
3. `run_rows_analysis(...)`
4. `run_table_analysis(...)`

## Rerank Boundary

If your workflow needs rerank data:

1. fetch it through `wendao-core-lib` or `wendao-arrow-interface`
2. keep rerank transport ownership there
3. hand the returned table into `analyze_table(...)` if you want Python-side
   post-analysis

There is no analyzer-owned rerank workflow in this package's beta contract.
