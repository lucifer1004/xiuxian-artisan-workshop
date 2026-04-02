---
type: knowledge
metadata:
  title: "External Consumer Checklist"
---

# External Consumer Checklist

Use this checklist before trying `xiuxian-wendao-analyzer` from another
workspace or as an early external user.

## Environment

Confirm:

1. Python `>=3.12`
2. `pyarrow>=14.0.0`
3. `xiuxian-wendao-py` is installable in the same environment
4. example commands are run through `uv run python ...` from this package
   directory unless you have already installed the package into another Python
   environment

The current package metadata for that baseline lives in:

1. [pyproject.toml](../pyproject.toml)

## Choose Your Workflow First

Pick one workflow before wiring code:

1. local rerank experiment
2. host-backed repo-search with built-in `score_rank`
3. host-backed repo-search with a custom Python analyzer
4. host-backed rerank exchange through a live runtime Flight host

Start from:

1. [first_analyzer_author_tutorial.md](./first_analyzer_author_tutorial.md)
2. [write_your_first_custom_analyzer.md](./write_your_first_custom_analyzer.md)

## Host-Backed Requirements

If you choose a host-backed repo-search workflow, confirm:

1. a reachable `wendao_search_flight_server`
2. the selected host port and schema version are known
3. repo-search data is available for the target repo

Smallest honest host-backed trial:

```bash
direnv exec . cargo build -p xiuxian-wendao --features julia --bin wendao_search_flight_server --bin wendao_search_seed_sample
tmp_root="$(mktemp -d)"
direnv exec . .cache/pyflight-f56-target/debug/wendao_search_seed_sample alpha/repo "$tmp_root"
direnv exec . .cache/pyflight-f56-target/debug/wendao_search_flight_server 127.0.0.1:8815 --schema-version=v2 alpha/repo "$tmp_root" 3
uv run python examples/repo_search_workflow.py --host 127.0.0.1 --port 8815 --query-text alpha --path-prefix src/
```

Reading of that sequence:

1. build the two Rust binaries once
2. seed a sample repo into a temporary workspace
3. boot a real `wendao_search_flight_server`
4. run the shipped repo-search analyzer example against that live host

If you want the same path in one command, use:

```bash
uv run python examples/host_backed_repo_search_beta_smoke.py --port 0
uv run python examples/host_backed_repo_search_beta_smoke.py --mode custom --port 0
uv run python examples/host_backed_repo_search_beta_smoke.py --port 0 --keep-workspace
```

Use `--keep-workspace` when you need to inspect the seeded sample repo or debug
the host-backed path after the smoke finishes.

Use these examples first:

1. [repo_search_workflow.py](../examples/repo_search_workflow.py)
2. [custom_repo_analyzer_workflow.py](../examples/custom_repo_analyzer_workflow.py)
3. [host_backed_repo_search_beta_smoke.py](../examples/host_backed_repo_search_beta_smoke.py)

## Host-Backed Rerank Requirements

If you choose a host-backed rerank exchange workflow, confirm:

1. a reachable `wendao_flight_server` or equivalent runtime host exposing
   `/rerank/flight`
2. your input rows already match `WendaoRerankRequestRow`
3. the selected host port and schema version are known

Start from:

1. `run_rerank_exchange_analysis(...)`
2. `summarize_rerank_exchange(...)`
3. [rerank_exchange_workflow.py](../examples/rerank_exchange_workflow.py)

If you want the same rerank path in one command, use:

```bash
uv run python examples/host_backed_rerank_beta_smoke.py --port 0
```

## Local-Only Requirements

If you choose a local rerank workflow, confirm:

1. your input rows already match rerank-shaped input
2. you do not need a live host for analyzer-owned rerank input

Start from:

1. [local_rerank_workflow.py](../examples/local_rerank_workflow.py)

## Known Skip Paths

You should stop and not expect success yet if you need:

1. a live-host custom Python analyzer rerank workflow
2. a plugin migration path out of `xiuxian-wendao-py`
3. GA-level compatibility guarantees for every helper-shaped symbol

## First Trial Goal

For the first external trial, the recommended success bar is small:

1. run one shipped example unchanged
2. run one host-backed or local workflow from the tutorials
3. only then start modifying analyzer logic

Recommended beta invocation:

```bash
uv run python examples/local_rerank_workflow.py
uv run python examples/repo_search_workflow.py --help
uv run python examples/custom_repo_analyzer_workflow.py --help
uv run python examples/rerank_exchange_workflow.py --help
uv run python examples/host_backed_rerank_beta_smoke.py --help
```

Do not assume plain `python examples/...` is the intended beta path unless the
package has already been installed into that interpreter environment.
