# Wendao Repo Done-Gate Recovery

## Purpose / Big Picture

Recover the remaining validation blockers surfaced after the repo compat-surface
retirement so the Wendao repo lane can pass its intended done gates and land as
one coherent change set.

## Scope and Boundaries

Files or dirs to read:
- `.cache/agent/blueprints/wendao_repo_substrate_extraction.md`
- `.cache/agent/execplans/archives/wendao-repo-compat-surface-retirement.md`
- `packages/rust/crates/xiuxian-wendao/tests/integration/repo_overview.rs`
- `packages/rust/crates/xiuxian-wendao/tests/integration/repo_sync.rs`
- `packages/rust/crates/xiuxian-wendao/src/gateway/studio/router/handlers/repo/analysis/search/`
- `packages/rust/crates/xiuxian-wendao/src/gateway/studio/search/handlers/tests/`
- `packages/rust/crates/xiuxian-wendao/src/search_plane/service/tests/`
- `packages/rust/crates/xiuxian-wendao/tests/unit/link_graph_agentic/`
- touched docs / GTD / tracking files only if the slice meaningfully changes status

Commands or tools to run:
- `ls`, `sed`, `nl`, `rg`
- `direnv exec . cargo test -p xiuxian-wendao --lib`
- `direnv exec . cargo test -p xiuxian-wendao <targeted filter> --lib`
- `direnv exec . cargo clippy -p xiuxian-wendao --lib --tests -- -D warnings`
- `direnv exec . cargo test -p xiuxian-git-repo --lib --tests`
- `direnv exec . git diff --check -- ...`

Expected outputs:
- repo integration snapshots normalize revision values instead of pinning unstable hashes
- repo search hot-cache regression is deterministic again
- repo-content and publication-state failing tests are green
- touched `xiuxian-wendao` test scope is clippy-clean

Stop conditions:
- stop if a blocker requires redesigning search-plane publication contracts
- stop if unrelated user edits appear in the failing files
- stop if a failure depends on nondeterministic external environment rather than repo code

## Context and Orientation

The compat-surface retirement itself compiled and passed targeted caller
validation, but full Wendao done gates still exposed pre-existing baseline
noise:

1. repo integration snapshots still pinned concrete revision hashes
2. one repo-analysis hot-cache test used a non-unique cache identity
3. one repo-content publication test failed on a schema mismatch
4. one search-plane publication-state test timed out
5. `cargo clippy -p xiuxian-wendao --lib --tests -D warnings` still reported
   test-only lint debt

## Plan of Work

1. Inspect the exact failing tests and helper seams.
2. Fix the blockers in the narrowest deterministic way.
3. Re-run the full and targeted gates, then commit if green enough for the lane.

## Validation and Acceptance

- `direnv exec . cargo test -p xiuxian-wendao --lib`
- `direnv exec . cargo clippy -p xiuxian-wendao --lib --tests -- -D warnings`
- `direnv exec . cargo test -p xiuxian-git-repo --lib --tests`
- `direnv exec . git diff --check -- .cache/agent/blueprints/wendao_repo_substrate_extraction.md .cache/agent/execplans/wendao-repo-done-gate-recovery.md docs/GTD/DAILY_2026_04_05.md packages/rust/crates/xiuxian-wendao packages/rust/crates/xiuxian-git-repo`

Acceptance:
- repo lane validation blockers are removed or explicitly bounded as non-lane issues
- the worktree contains no snapshot `.new` noise
- the slice is ready to commit intentionally

## Outcome

Status: `[DONE]`

Recovered blockers:

1. repo gateway cache tests now use temp-root-derived unique keyspaces
2. repo runtime tests now have an awaitable synchronization helper so
   manifest/runtime hydration assertions do not race background refresh
3. repo sync and overview gateway snapshots now normalize revision hashes
4. touched link-graph agentic test helpers are clippy-clean without adding
   blanket lint suppression

Validation completed:

- `direnv exec . cargo fmt --all`
- `direnv exec . cargo test -p xiuxian-wendao --lib`
- `direnv exec . cargo clippy -p xiuxian-wendao --lib --tests -- -D warnings`
- `direnv exec . cargo test -p xiuxian-git-repo --lib --tests`
- `direnv exec . git diff --check -- packages/rust/crates/xiuxian-wendao packages/rust/crates/xiuxian-git-repo .cache/agent/execplans/wendao-repo-done-gate-recovery.md docs/GTD/DAILY_2026_04_05.md`

## Interfaces and Dependencies

- Governing blueprint:
  `.cache/agent/blueprints/wendao_repo_substrate_extraction.md`
- Related archived ExecPlan:
  `.cache/agent/execplans/archives/wendao-repo-compat-surface-retirement.md`
