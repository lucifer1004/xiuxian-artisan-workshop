---
type: knowledge
metadata:
  title: "Wendao Flight Runtime Host Settings Contract"
---

# Wendao Flight Runtime Host Settings Contract

This document defines the current precedence contract for runtime-owned Flight
host settings that affect the Wendao query and rerank surface.

It exists to keep host-setting rules in one place so new runtime knobs do not
scatter precedence semantics across README text, RFC prose, GTD notes, and
ExecPlan entries.

## Contract Status

| Knob                | Owner                         | Status              | Notes                                                                      |
| ------------------- | ----------------------------- | ------------------- | -------------------------------------------------------------------------- |
| rerank weights      | `wendao_search_flight_server` | promoted            | Governed by this contract and backed by real-host conflict tests.          |
| schema version      | `wendao_search_flight_server` | promoted            | Governed by this contract and backed by real-host precedence tests.        |
| schema version      | `wendao_flight_server`        | promoted            | Governed by this contract and backed by runtime-host precedence tests.     |
| rerank dimension    | `wendao_flight_server`        | promoted            | Governed by this contract and backed by runtime-host precedence tests.     |
| rerank dimension    | `wendao_search_flight_server` | promoted            | Governed by this contract and backed by SearchPlane-host precedence tests. |
| any other host knob | any                           | implementation-only | Not governed precedence until it passes promotion.                         |

## Explicit Non-Promoted Inputs

The following inputs are intentionally treated as host bring-up parameters or
fixture controls, not governed precedence knobs:

| Input                                      | Owner                         | Status              | Reason                                                          |
| ------------------------------------------ | ----------------------------- | ------------------- | --------------------------------------------------------------- |
| `bind_addr`                                | both hosts                    | implementation-only | Transport bring-up parameter, not retrieval policy.             |
| `repo_id`                                  | `wendao_search_flight_server` | implementation-only | Host fixture/selection input, not shared runtime policy.        |
| `project_root` / current working directory | `wendao_search_flight_server` | implementation-only | Workspace resolution input, not a governed host-setting ladder. |
| `WENDAO_BOOTSTRAP_SAMPLE_REPO`             | `wendao_search_flight_server` | implementation-only | Test/bootstrap control only; no production policy semantics.    |

## Current Contract

### `wendao_search_flight_server`

- rerank weights:
  `workspace wendao.toml > process env > default`
- schema version:
  `explicit CLI override > workspace wendao.toml > default`
- rerank dimension:
  `explicit CLI override > positional arg > default`

### `wendao_flight_server`

- schema version:
  `explicit CLI override > default`
- rerank dimension:
  `explicit CLI override > positional arg > default`

## Promotion Rules

A runtime knob should only enter this contract when all of the following are
true:

1. the knob changes real host behavior rather than only test-local behavior
2. the precedence is explicit in implementation, not accidental
3. the precedence is validated on a live host path
4. the precedence is documented in the successor RFC

## Promotion Checklist

Before adding a new runtime host knob to this contract, confirm:

- the knob affects a live Flight host and not only an internal helper
- the knob has a clear owner:
  `wendao_search_flight_server`, `wendao_flight_server`, or both
- the precedence ladder is fully specified in one line
- the default behavior is explicit
- conflict behavior is covered by a real-host test when multiple sources exist
- README, RFC, and planning docs only reference this contract instead of
  duplicating the full precedence table
- the knob adds real operational value rather than cosmetic symmetry

## Documentation Rule

When a new runtime host knob is promoted into contract status:

1. update this document first
2. update the successor RFC to reference this contract
3. update package README or GTD only with the delta, not a duplicate full list

## Validation Rule

A precedence claim is not considered landed until a real-host validation path
proves the conflict or override behavior end to end.

## Interpretation Rule

If a knob is not listed in the contract-status table as `promoted`, it must be
treated as implementation detail rather than stable host-settings contract.

## Current Closure

At the current host surface, there is no additional high-value runtime knob
that clearly satisfies promotion requirements beyond:

- rerank weights
- schema version
- rerank dimension

Remaining exposed inputs are currently bring-up, workspace-resolution, or
fixture controls and should remain implementation-only unless their semantics
change materially.
