---
type: knowledge
title: "RFC 0007: Flowchart Backbone Minimal Rules for Bounded Plan Work"
category: "rfc"
status: "draft"
authors:
  - codex
created: 2026-04-08
tags:
  - rfc
  - qianji
  - flowchart
  - planning
  - mermaid
  - cli
---

# RFC 0007: Flowchart Backbone Minimal Rules for Bounded Plan Work

## 1. Summary

This RFC defines the minimum visible backbone that `flowchart.mmd` must expose
for a bounded plan work surface.

Version one keeps this deliberately small:

1. `flowchart.mmd` is the direct graph companion file
2. the principal bounded surfaces must be visible in the flowchart
3. the backbone direction between those surfaces must be visible
4. `qianji check` should reject obvious backbone conflicts, not require full
   graph isomorphism

The goal is to keep `flowchart.mmd` useful for humans and Codex without
turning it into a second heavyweight contract language.

## 2. Context

Earlier RFCs already define:

1. the bounded work surface shape
2. the compact `[plan]` plus `[check]` manifest
3. `qianji show` as the graph and contract display surface
4. `qianji check` as the unified validation and guard surface

What still needs to be fixed is the smallest version-one answer to this
question:

> what must `flowchart.mmd` visibly contain before the bounded plan work
> surface is considered aligned?

This RFC answers only that question.

## 3. Non-Goals

This RFC does not define:

1. full Mermaid syntax validation
2. full graph equivalence between `qianji.toml` and `flowchart.mmd`
3. rendering rules beyond backbone visibility
4. retrieval semantics
5. a requirement to show every internal node or checklist item

## 4. Backbone Contract

For version one, `flowchart.mmd` is valid when it makes the principal bounded
surfaces and their backbone direction visible.

For the minimal bounded plan work surface, those principal surfaces are:

1. `blueprint`
2. `plan`

If the local contract declares:

```toml
[check]
flowchart = ["blueprint", "plan"]
```

then `flowchart.mmd` must visibly expose those surfaces and their backbone
relationship.

## 5. Minimum Visible Rules

### 5.1 File Presence

`flowchart.mmd` must exist at the bounded work-surface root.

### 5.2 Principal Surface Visibility

The flowchart must visibly contain each principal surface named by
`[check].flowchart`.

For version one, visibility means that the surface can be recognized as a
Mermaid node label or node identifier in the graph companion file.

### 5.3 Backbone Direction Visibility

The flowchart must visibly express the direction of the backbone between the
principal surfaces.

For the minimal work surface, that means:

```mermaid
flowchart LR
  blueprint --> plan
```

or a richer upstream graph that still preserves the same direction, for
example:

```mermaid
flowchart LR
  coding --> rust
  rust --> blueprint
  blueprint --> plan
```

### 5.4 No Obvious Backbone Conflict

`qianji check` must fail if the visible backbone clearly conflicts with the
bounded contract.

Examples of obvious conflict:

1. `plan --> blueprint` when the contract expects `blueprint --> plan`
2. `blueprint` is present but `plan` is absent
3. `plan` is present but disconnected from the declared backbone

## 6. Allowed Compression

Version one should allow `flowchart.mmd` to be smaller than the full internal
work graph.

This means the flowchart may:

1. omit lower-level markdown sections
2. omit internal checklist nodes
3. omit derived tracking details such as `plan-track`
4. compress multiple internal details into one visible backbone node

This means the flowchart must not:

1. hide the principal surfaces named by `[check].flowchart`
2. reverse the declared backbone direction
3. replace the backbone with unrelated labels that make the work surface
   unrecognizable

## 7. `plan-track` Visibility

`plan-track` remains the derived tracking face of `plan`, but version one does
not require it to appear as a principal node inside `flowchart.mmd`.

It may appear when useful, but the minimal visible backbone still centers on
the user-facing bounded work surfaces:

1. `blueprint`
2. `plan`

This keeps the graph companion small while leaving tracking behavior under
`qianji check`.

## 8. Relation to `qianji show`

`qianji show --dir <plan-workdir>` should treat `flowchart.mmd` as the first
graph entry surface.

Its responsibility is to expose:

1. the graph companion file
2. the top-level `blueprint/` surface
3. the top-level `plan/` surface

It should not reinterpret the flowchart into a richer retrieval surface.

## 9. Relation to `qianji check`

`qianji check --dir <plan-workdir>` should use this RFC as the minimum
flowchart-alignment contract.

Version one should check:

1. `flowchart.mmd` exists
2. every principal surface named by `[check].flowchart` is visibly present
3. the visible backbone direction is compatible with the declared contract
4. no obvious conflict blocks continued bounded work

Version one should not require a full parse-equivalent graph comparison.

## 10. Minimum Accepted Examples

### 10.1 Accepted Minimal Backbone

```mermaid
flowchart LR
  blueprint --> plan
```

### 10.2 Accepted Richer Upstream Backbone

```mermaid
flowchart LR
  coding --> rust
  rust --> blueprint
  blueprint --> plan
```

### 10.3 Rejected Reversed Backbone

```mermaid
flowchart LR
  plan --> blueprint
```

### 10.4 Rejected Missing Principal Surface

```mermaid
flowchart LR
  blueprint --> delivery
```

## 11. Design Rationale

The bounded work surface needs a graph companion that is:

1. small enough for quick inspection
2. stable enough for `qianji check`
3. expressive enough for Codex to orient before Wendao retrieval

The smallest rule set that achieves that is:

1. require the named principal surfaces
2. require the visible backbone direction
3. reject only obvious conflicts in version one

Anything heavier would turn `flowchart.mmd` into a second full contract
language, which this RFC rejects.

## 12. Rejected Alternatives

This RFC rejects:

1. requiring full graph equivalence between `qianji.toml` and `flowchart.mmd`
2. requiring every internal markdown unit to appear in the Mermaid graph
3. requiring `plan-track` to be a user-visible principal node in version one
4. treating `flowchart.mmd` as a retrieval surface rather than a graph
   companion surface

## 13. Conclusion

Version one should keep the flowchart contract narrow:

1. principal surfaces must be visible
2. the backbone direction between them must be visible
3. obvious conflicts must fail `qianji check`
4. everything else may remain compressed

This is enough to keep `flowchart.mmd` aligned with the bounded plan work
surface without creating a second heavy graph language.
