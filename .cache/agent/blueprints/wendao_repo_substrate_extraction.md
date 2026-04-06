# Wendao Repo Substrate Extraction

> This blueprint governs the extraction of repository materialization,
> revision-resolution, and checkout synchronization infrastructure out of
> `xiuxian-wendao` and into a dedicated reusable crate.

- Owner:
- Status: active
- Workstream:
  - `wendao/repo_substrate_extraction`
- Governs:
  - `packages/rust/crates/xiuxian-wendao/src/analyzers/repo_source.rs`
  - `packages/rust/crates/xiuxian-git-repo/`
- Time horizon:
  - next 1-3 implementation slices only
- Governing ExecPlans:
  - `.cache/agent/execplans/wendao-repo-substrate-gix-cutover.md`
  - `.cache/agent/execplans/wendao-repo-remote-config-gix-hardening.md`
  - `.cache/agent/execplans/archives/wendao-repo-remote-probe-gix-cutover.md`
  - `.cache/agent/execplans/archives/wendao-repo-mutation-gix-cutover.md`
  - `.cache/agent/execplans/archives/wendao-repo-final-gix-cutover.md`
  - `.cache/agent/execplans/archives/wendao-repo-checkout-safety-hardening.md`
  - `.cache/agent/execplans/archives/wendao-repo-compat-surface-pruning.md`
  - `.cache/agent/execplans/archives/wendao-repo-runtime-git2-retirement.md`
  - `.cache/agent/execplans/archives/wendao-repo-compat-surface-retirement.md`
  - `.cache/agent/execplans/archives/wendao-modelica-git2-test-retirement.md`
- Related stable references:
  - `packages/rust/crates/xiuxian-wendao/docs/06_roadmap/404_repo_intelligence_for_sciml_and_msl.md`
  - `packages/rust/crates/xiuxian-wendao/README.md`

## 1. Architectural Objective

The current Wendao crate still owns generic repository substrate concerns:
managed mirror layout, remote revision probing, checkout locking, retry
orchestration, and local checkout metadata inspection. Those responsibilities
are reused by repository intelligence flows but are not themselves search or
graph domain logic.

This workstream exists to make the repository substrate reusable,
backend-agnostic at the public API, and no longer owned by `xiuxian-wendao`.
The architectural condition that must become true is:

1. repository materialization and revision synchronization live in a dedicated
   crate
2. Wendao depends on that crate instead of exposing or depending on raw git
   backend handles
3. retry and failure policy are driven by crate-owned taxonomy rather than
   backend-specific string matching at the outer layers

This belongs in a blueprint because the crate boundary, ownership rules, and
failure contracts must remain stable across multiple migration slices.

## 2. System Context and Design Drivers

### 2.1 Current Reality

`xiuxian-wendao` previously exposed `src/git/checkout/` as a feature-folder
surface, but the implementation depended directly on `git2::Repository`,
`git2::FetchOptions`, `git2::RepoBuilder`, and backend-specific error strings.
That meant Wendao domain code was coupled to one git transport library and
owned a generic runtime helper that other crates could reuse. As of the latest
retirement slice, `src/git/` is gone and Wendao keeps only a small
`RegisteredRepository` adapter under `src/analyzers/repo_source.rs`.

### 2.2 Design Drivers

1. package ownership must keep `xiuxian-wendao` focused on Wendao business
   domain behavior instead of generic repository runtime helpers
2. public interfaces must stay stable even if the internal git backend changes
3. the repo layout must remain compatible with the existing ghq-style
   materialization policy
4. checkout locking, retry, and health reporting must remain explicit and
   testable
5. migration risk must stay bounded by preserving behavior through contract
   tests while the implementation moves

## 3. Target Architecture

### 3.1 Target Shape

`packages/rust/crates/xiuxian-git-repo/` becomes the canonical owner of repo
substrate responsibilities:

1. repo specs and revision selectors
2. ghq-style mirror and checkout layout
3. managed mirror clone/fetch/probe/checkout synchronization
4. checkout lock lifecycle and stale-lock reclamation
5. repo metadata, source observation, and raw document collection
6. repo health, sync outcome, and retryable failure classification

`xiuxian-wendao` consumes those capabilities through crate-owned types and
services, not through backend handles.

### 3.2 Responsibilities and Ownership

- `xiuxian-git-repo`
  - owns repository substrate types, sync services, backend adapters,
    retry/error taxonomy, and contract tests
- `xiuxian-wendao`
  - owns repository-intelligence domain flows, analyzers, entity/content
    projection, and any search-plane consumption of repo substrate outputs
- `xiuxian-testing`
  - owns reusable test gates/helpers when repo substrate tests need shared
    validation infrastructure

### 3.3 Interfaces and Data Contracts

The public API must expose crate-owned contracts only, including:

1. `RepoSpec`
2. `RevisionSelector`
3. `MaterializedRepo`
4. `RepoInventoryEntry`
5. `RawRepoDocument`
6. `SyncOutcome`
7. `RepoHealth`
8. `RepoError` and `RepoErrorKind`

Public interfaces must not expose `git2::*` or `gix::*` repository types.

### 3.4 Failure Model and Operational Expectations

The crate must classify operational failures into a stable taxonomy so retry
policy does not depend on outer-layer message matching. The runtime must retain
explicit handling for transient network issues, descriptor pressure, lock
contention, revision lookup failures, authentication failures, misconfigured
remotes, and repository corruption. Materialization paths must continue using
the ghq-style layout under project data roots.

## 4. Design Principles and Invariants

### 4.1 Wendao Does Not Own Generic Repo Runtime

New repository substrate logic must land in `xiuxian-git-repo`, not back inside
`xiuxian-wendao`, unless it is clearly search-domain specific.

### 4.2 Public API Is Backend-Neutral

No public surface may leak backend repository handles or backend-specific error
types.

### 4.3 Gix Is The Only Formal Backend Direction

The new crate is gix-first. Temporary migration scaffolding may exist only when
required to keep a bounded slice moving, but the blueprint does not authorize a
long-lived dual-backend public design.

### 4.4 Layout Compatibility Stays Stable

The managed repo layout must preserve the existing ghq-style root mapping so
existing configured repositories do not drift onto incompatible paths.

### 4.5 Contract Tests Guard Behavior

Behavioral parity must be proven through repo-substrate contract tests around
clone/fetch/probe/revision resolution/locking/retry rather than by assuming API
similarity between backends.

## 5. Governed Boundaries

### 5.1 Ownership Boundaries

This blueprint governs repository substrate extraction only. It does not change
ownership of analyzer logic, repo-entity projection, search ranking, or gateway
transport layers.

### 5.2 Protocol Boundaries

In scope:

1. local and managed repo materialization contracts
2. repo sync health/status contracts
3. repo metadata and raw document observation contracts
4. retry/error classification contracts

### 5.3 Package or Subsystem Boundaries

The concrete boundary is between `packages/rust/crates/xiuxian-wendao/src/git/`
and the new `packages/rust/crates/xiuxian-git-repo/` crate.

### 5.4 Non-Goals

This blueprint does not attempt to:

1. move repo entity/content indexing into the new crate
2. redesign search-plane publication or gateway routes
3. migrate every existing `git2`-using test helper in one slice
4. solve all gix optimization work before the first crate extraction lands

## 6. Key Decisions and Rejected Alternatives

### 6.1 Chosen Direction

Extract a dedicated `xiuxian-git-repo` crate, make its public API
backend-neutral, and implement the substrate against `gix` as the formal
backend direction.

### 6.2 Rejected or Deferred Alternatives

1. keep the substrate inside `xiuxian-wendao`
   - rejected because it preserves the wrong ownership boundary
2. expose raw backend repository handles
   - rejected because it locks downstream crates to one implementation
3. build a long-lived `git2` and `gix` dual-backend product surface
   - deferred/rejected because current steering explicitly does not want to
     over-preserve `git2`

### 6.3 Revisit Triggers

Revisit this design if:

1. `gix` cannot provide required clone/fetch/probe semantics for the current
   operating envelope
2. the ghq-style layout needs a different materialization policy
3. another crate needs a narrower split between sync substrate and document
   observation than the current crate shape provides

## 7. Immediate Evolution Slices

### Slice 1

- architectural purpose: create `xiuxian-git-repo`, move the current repo
  substrate boundary out of `xiuxian-wendao`, and retarget Wendao callers to
  the new crate without widening into search-domain code
- prerequisite dependencies or proofs: current `src/git/checkout/*` ownership
  map, crate/workspace wiring, and focused regression coverage for checkout
  metadata plus local/managed sync behavior
- expected completion signal: `xiuxian-wendao` no longer owns the extracted
  substrate modules and focused tests pass against the new crate

### Slice 2

- architectural purpose: harden the gix-first backend path and complete the
  backend-neutral error taxonomy/retry model
- prerequisite dependencies or proofs: slice-1 cutover complete with contract
  tests in place
- expected completion signal: retry and health reporting are crate-owned and do
  not depend on outer-layer string matching

### Slice 3

- architectural purpose: finish downstream call-site cleanup and remove any
  temporary bridge surfaces still tying Wendao tests or adapters to the old
  module layout
- prerequisite dependencies or proofs: slices 1-2 stable
- expected completion signal: `xiuxian-wendao` has no long-lived repo
  substrate residue beyond domain-level consumption

## 8. Risks, Unknowns, and Required Evidence

### 8.1 Risks and Unknowns

1. the current substrate may rely on `git2` semantics that need explicit gix
   adaptation
2. large worktree churn increases the risk of accidental overlap with unrelated
   parser/julia edits
3. repo test helpers may still assume `git2` during the first slice

### 8.2 Required Evidence

1. focused contract tests for metadata, materialization, retry, and locks
2. workspace build proof showing the new crate is wired correctly
3. package-doc and GTD updates that record the new ownership boundary
4. explicit validation notes for any remaining blockers or deferred follow-up

## 9. Current Implementation Note

As of 2026-04-05, the governed production substrate has crossed the final
backend cutover:

1. `xiuxian-git-repo` no longer carries a production native-`git` bridge
2. detached checkout now executes through `gix` index/worktree/reference
   mutation with stale tracked-path pruning parity
3. the internal `gix` backend has been modularized into
   `src/backend/gix/{retry,open,probe,remote,clone,fetch,checkout}.rs`
4. detached checkout stale-path cleanup now refuses recursive directory
   removal when an unexpected directory collision could delete unrelated
   untracked contents

The blueprint remains active only for any future downstream ownership cleanup.

As of the latest compat-surface retirement slice:

5. `xiuxian-wendao/src/git/` is deleted
6. Wendao callers now consume `xiuxian-git-repo` contracts directly
7. the only remaining Wendao-local bridge is
   `src/analyzers/repo_source.rs` for config/error adaptation
8. the immediate post-retirement done-gate recovery is complete, including
   deterministic repo cache/runtime tests, backend-neutral revision snapshots,
   and green Wendao lib-test plus clippy verification for the lane

## 9. Alignment Audit Rules

An ExecPlan claims alignment only if:

1. the new crate, not Wendao, remains the target owner of repo substrate logic
2. public types remain backend-neutral
3. ghq-style layout compatibility stays intact
4. search-domain or gateway work is not silently folded into the substrate lane
5. execution details, command logs, and slice-local blockers stay in the
   ExecPlan rather than expanding the blueprint into a task log
