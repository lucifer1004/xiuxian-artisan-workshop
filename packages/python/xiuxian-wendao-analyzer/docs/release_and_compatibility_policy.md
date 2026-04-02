---
type: knowledge
metadata:
  title: "Release and Compatibility Policy"
---

# Release and Compatibility Policy

This package is currently in beta.

The goal of this policy is to state what external users may rely on today, and
what may still change.

## Current Stability Reading

`xiuxian-wendao-analyzer` is implemented and usable, but not yet a frozen
general-availability surface.

The current lockable beta baseline is `0.1.1`.

External users may rely on:

1. the package existing as a separate dependency from `xiuxian-wendao-py`
2. the documented workflow split between:
   - host-backed repo-search analysis
   - host-backed rerank exchange analysis
   - local rerank analysis
3. the current example entrypoints under `examples/`
4. the current tutorial set under `docs/`

External users should not yet assume:

1. a stable plugin migration story
2. a live-host custom Python analyzer rerank workflow above the current
   transport route
3. a permanent guarantee for every helper-shaped convenience symbol

## Compatibility Rule For This Beta

During the current beta phase:

1. breaking changes to documented workflows should be treated as high-cost
2. changes to runnable examples and tutorials must stay synchronized
3. helper additions are allowed, but helper churn should not invalidate the
   documented workflow entrypoints without an explicit migration note

The practical contract is workflow-first:

1. prefer keeping `run_repo_analysis(...)`-based repo workflows stable
2. prefer keeping `run_rerank_exchange_analysis(...)`-based host-backed rerank
   workflows stable
3. prefer keeping `run_rerank_analysis(...)`-based local workflows stable
4. treat lower-level helper reshaping as less stable than the top-level
   documented workflows

For the current locked beta baseline:

1. package-root version export is part of the public package boundary
2. repo-search examples and tests remain aligned with the `schema_version=v2`
   Flight contract
3. host-backed rerank exchange remains aligned with the live `/rerank/flight`
   seam owned below this package

## What A Future Freeze Would Need

The package should not claim broader compatibility guarantees until at least:

1. a release policy is written for versioning and change notes
2. the supported public workflow entrypoints are explicitly frozen
3. the current beta gaps are either closed or intentionally accepted

## Current Beta Exit Reading

The package is close to a stable beta workflow boundary, but not yet at a
general freeze point.

Exit-ready evidence already exists for:

1. host-backed repo-search with built-in ranking
2. host-backed repo-search with a custom Python analyzer
3. host-backed rerank exchange analysis through a live `/rerank/flight` route
4. local rerank analysis over typed rows and Arrow tables
5. shipped examples and one-shot beta smoke coverage for the current repo paths
6. substrate-level live `/rerank/flight` exchange through `xiuxian-wendao-py`

Exit blockers still remain for:

1. live-host custom Python analyzer rerank workflows
2. plugin migration and compatibility guidance
3. a broader public API freeze beyond workflow-first guarantees

The current reading should stay workflow-first:

1. beta exit should be judged by missing external workflows
2. beta exit should not be judged by raw helper count or helper symmetry

## Current Beta Freeze Reading

The current beta should be treated as workflow-frozen, not helper-frozen.

Frozen for this beta:

1. the documented workflow split:
   - host-backed repo-search analysis
   - host-backed rerank exchange analysis
   - local rerank analysis
2. the shipped example set under `examples/`

Not frozen for this beta:

1. lower-level helper reshaping below the documented workflows
2. live-host custom Python analyzer rerank workflows
3. plugin migration semantics

The practical freeze rule remains narrow:

1. do not widen the workflow set without a new external-user workflow gap
2. do not add example variants just for symmetry
3. prefer keeping the current workflow and example set coherent over growing it

## Related Documents

1. package overview: [README.md](../README.md)
2. onboarding tutorial: [first_analyzer_author_tutorial.md](./first_analyzer_author_tutorial.md)
3. custom analyzer tutorial: [write_your_first_custom_analyzer.md](./write_your_first_custom_analyzer.md)
