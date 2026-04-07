# Config Import Overlay

## Purpose

Wendao configuration now resolves through one merge engine:
`xiuxian-config-core` owns recursive `imports = [...]`, import-path
environment expansion, and merge order. Wendao and gateway consumers project
typed runtime values from that merged TOML instead of re-implementing line
scanners or local deep-merge helpers.

## Merge Model

The merge order is:

1. embedded defaults from `resources/config/wendao.toml`
2. boot-time base config from `$PRJ_ROOT/wendao.toml` or an explicit override
   path
3. optional Studio overlay wrapper from `wendao.studio.overlay.toml`

`imports` are merged before the importing file body, so the importing file wins
on conflicts. That means the Studio wrapper must import the base file:

```toml
imports = ["wendao.toml"]

[link_graph.projects.main]
root = "."
dirs = ["docs", "src"]
```

The reverse shape would be wrong because `wendao.toml` would overwrite the
overlay on merge.

## Effective Config Paths

- the checked-in workspace boot config now lives at `$PRJ_ROOT/wendao.toml`
- shared runtime settings in `src/settings/` resolve through
  `xiuxian-config-core`
- gateway CLI config resolution prefers the sibling
  `wendao.studio.overlay.toml` wrapper when it exists
- Studio bootstrap config loading reads the same effective path
- repo-intelligence config loading is import-aware, so base-plus-overlay repo
  registrations use the same merged TOML contract
- `nix/modules/process.nix` now points at the root config and uses a bounded
  TOML-aware helper under `scripts/channel/` for readiness-port discovery

## Environment Variables In Import Paths

`xiuxian-config-core` now expands environment variables inside `imports`
entries. Supported forms are `$VAR` and `${VAR}`.

The intended use is path-like import composition, for example:

```toml
imports = ["${PRJ_ROOT}/packages/rust/crates/xiuxian-wendao/resources/config/wendao.toml"]
```

This support is intentionally limited to import paths. Wendao does not treat
arbitrary string values in TOML as general environment-template fields.

## Studio Overlay Persistence

`POST /api/ui/config` now persists the live Studio state as
`wendao.studio.overlay.toml` instead of mutating the base `wendao.toml`
directly.

The overlay writer preserves the base file and uses empty-array tombstones to
shadow UI-owned entries that should disappear on the next boot:

- `dirs = []` suppresses a base local-project entry
- `plugins = []` suppresses a base repo-project entry

That keeps restart behavior aligned with the effective UI state while leaving
non-UI base sections such as `[gateway]` untouched.

## Ownership Boundary

- `xiuxian-config-core` owns import recursion, path expansion, and merge
  semantics
- `xiuxian-wendao/src/settings/` owns conversion from merged TOML into Wendao
  runtime settings
- `gateway/studio/router/config/` owns overlay file naming and `UiConfig`
  projection
- `src/bin/wendao/execute/gateway/config.rs` consumes merged gateway settings
  instead of scanning TOML text
