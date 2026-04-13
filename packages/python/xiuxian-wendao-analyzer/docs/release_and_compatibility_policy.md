# Release And Compatibility Policy

This package is currently in beta.

## Compatibility Rule For This Beta

The current lockable beta baseline is `0.2.1`.

The compatibility promise is workflow-frozen, not helper-frozen.

That means we protect the documented workflow set:

1. offline repo-search authoring with `WendaoArrowSession.for_repo_search_testing(...)`
2. scripted PDF attachment search with `WendaoArrowSession.attachment_search(...)`
3. host-backed repo-search analysis with `run_repo_analysis(...)`
4. host-backed repo-search analysis with a custom analyzer object
5. generic rows, table, and query analysis over Rust-returned data

It does not mean a permanent guarantee for every helper-shaped convenience
symbol that previously appeared during beta exploration.

## Boundary Stability

The current beta rule is:

1. Rust and transport packages own rerank transport behavior
2. `xiuxian-wendao-analyzer` owns analysis over returned rows and tables
3. analyzer-owned rerank helpers are out of scope for this beta baseline

If you need rerank data, use the substrate-level transport in `wendao-core-lib`
or the facade in `wendao-arrow-interface`, then analyze the returned table with
generic analyzer helpers.

## Current Beta Freeze Reading

Frozen now:

1. the repo-search workflows documented in the README and tutorials
2. the five shipped examples under `examples/`
3. `AnalyzerConfig(strategy="score_rank")` as the built-in analyzer strategy

Not frozen now:

1. new helper symmetry for every possible Wendao route
2. future analyzer strategies beyond `score_rank`
3. any previously exposed beta-only rerank helper that conflicts with the
   Rust-query-first analyzer boundary

## Current Beta Exit Reading

The package is usable now, but not yet GA:

1. repo-search host-backed validation exists
2. offline scripted authoring exists
3. docs now align with the Rust-query-first analyzer ownership model

Remaining beta gaps:

1. no GA-level release promise yet
2. no broad downstream compatibility window yet
3. no guarantee that new workflows will be added without version movement
