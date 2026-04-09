---
type: knowledge
title: "RFC 0003: Wendao SQL Minimal Retrieval Surface for Bounded Plan Work"
category: "rfc"
status: "draft"
authors:
  - codex
created: 2026-04-07
tags:
  - rfc
  - qianji
  - wendao
  - sql
  - retrieval
  - planning
---

# RFC 0003: Wendao SQL Minimal Retrieval Surface for Bounded Plan Work

## 1. Summary

This RFC defines the minimal retrieval surface for bounded plan work.

Its core rules are:

1. `qianji` does not define a new query language
2. exact content retrieval is provided through Wendao SQL
3. version one defines only a minimal markdown retrieval surface to avoid
   syntax inflation

The goal is not to build a large retrieval system. The goal is to give Codex a
small, stable, direct retrieval surface that fits the bounded work model.

## 2. Background

In the current planning model:

1. `qianji show` exposes the first-order structural surface
2. `qianji check` enforces boundary, drift, status, and structural legality
3. `tree` only helps Codex decide whether deeper inspection is necessary
4. actual content retrieval is owned by Wendao

If `qianji.toml` also tried to define `[query]` or `[[query.lane]]`, the
system would add syntax growth and blur responsibility boundaries.

The current workspace already includes DataFusion-oriented query
infrastructure, so SQL is the natural retrieval surface.

## 3. Non-Goals

This RFC does not define:

1. a custom query DSL
2. Qianji-native retrieval syntax
3. a large SQL catalog
4. a repository-wide universal knowledge schema
5. vector-retrieval syntax
6. complex join contracts

Version one only addresses markdown retrieval for bounded plan work surfaces.

## 4. Responsibility Split

### 4.1 Qianji

Qianji owns:

1. `show`
2. `check`
3. graph contracts
4. boundary and drift constraints

### 4.2 Wendao

Wendao owns:

1. SQL retrieval
2. markdown structural projection
3. content surfaces such as `skeleton` and `body`

### 4.3 Codex

Codex should:

1. inspect structure first
2. decide whether deeper inspection is needed
3. use Wendao SQL to retrieve exact fragments
4. edit only inside the bounded work surface

## 5. Retrieval Principles

### 5.1 First-Order Structure Comes First

Codex should first inspect:

1. `qianji.toml`
2. `flowchart.mmd`
3. `blueprint/`
4. `plan/`

Only then should it decide whether deeper content retrieval is necessary.

### 5.2 `tree` Is Only a Structural Probe

`tree` is not the retrieval mechanism.

Its role is only to help Codex decide:

1. whether deeper inspection is necessary
2. whether `blueprint/` or `plan/` should be inspected next
3. whether the current top-level layout already reveals enough

### 5.3 Exact Retrieval Uses SQL

Deeper content retrieval is performed through:

```bash
wendao sql "<SQL>"
```

## 6. Minimal SQL Surface

Version one defines one logical table:

```text
markdown
```

This table serves markdown retrieval inside bounded plan work surfaces.

## 7. `markdown` Table Contract

### 7.1 Row Model

Each row in `markdown` represents a retrievable markdown structural unit.

That unit may be:

1. a document root
2. a heading subtree
3. a section that can be rendered into a stable `skeleton`

This RFC does not force a single internal AST granularity. It only requires a
stable external SQL surface.

### 7.2 Minimal Column Set

Version one requires these columns.

#### `path`

Type: string

Meaning: the markdown file path that owns this structural unit

Examples:

```text
blueprint/blueprint.md
plan/plan.md
```

#### `surface`

Type: string

Meaning: the top-level bounded surface that owns this structural unit

Allowed values:

1. `blueprint`
2. `plan`

#### `heading_path`

Type: string

Meaning: the heading path for the structural unit

Examples:

```text
Blueprint
Blueprint/Boundary
Plan
Plan/Rust
```

Document-root rows may use an empty string or another stable root marker.

#### `title`

Type: string

Meaning: the title text for the current structural unit

#### `level`

Type: integer

Meaning: the heading depth for the current structural unit

Document-root rows may use any stable implementation-defined value.

#### `skeleton`

Type: string

Meaning: the structural compression view for the current structural unit

This is not a summary. It should preserve:

1. headings
2. key property fields
3. checklist shape
4. key task outline

`skeleton` is the default surface Codex should read first.

#### `body`

Type: string

Meaning: the original body or fully expanded content for the current
structural unit

`body` is the second-level read surface. Codex should read it only when
`skeleton` is not enough.

## 8. `skeleton` Semantics

`skeleton` must satisfy three rules.

### 8.1 It Is Not a Summary

It must not collapse the source into a short explanation. It must preserve the
original structural shape.

### 8.2 It Preserves Structure

At minimum, it should preserve:

1. headings
2. key property-drawer fields
3. checklist outline
4. child-heading relationships

### 8.3 It Supports Low-Token Reading

Codex should default to reading `skeleton` before `body`.

## 9. Minimal Query Examples

### 9.1 Inspect Blueprint Skeletons

```bash
wendao sql "
select path, heading_path, skeleton
from markdown
where surface = 'blueprint'
order by path, heading_path
"
```

### 9.2 Inspect Plan Skeletons

```bash
wendao sql "
select path, heading_path, skeleton
from markdown
where surface = 'plan'
order by path, heading_path
"
```

### 9.3 Read the Full Body of One Section

```bash
wendao sql "
select path, heading_path, body
from markdown
where path = 'plan/plan.md'
  and heading_path = 'Plan/Rust'
"
```

### 9.4 Search for Boundary Sections

```bash
wendao sql "
select path, heading_path, skeleton
from markdown
where heading_path like '%Boundary%'
order by path, heading_path
"
```

## 10. Recommended Codex Read Order

This RFC recommends a fixed read order.

### Step 1

```bash
qianji show --dir <plan-workdir>
```

### Step 2

If needed, use `tree` only to decide whether deeper inspection is necessary.

### Step 3

Read `skeleton` first.

For example:

```bash
wendao sql "
select path, heading_path, skeleton
from markdown
where surface in ('blueprint', 'plan')
order by path, heading_path
"
```

### Step 4

Read `body` only when necessary.

## 11. Relation to `qianji.toml`

`qianji.toml` does not define retrieval syntax.

It defines only:

1. the bounded surface
2. the `show` surface
3. `[[validation]]`

Therefore:

1. `qianji.toml` is the contract
2. Wendao SQL is retrieval
3. `tree` is structural probing
4. `qianji check` is the legality gate

These responsibilities must stay separate.

## 12. Relation to `qianji check`

`qianji check` does not perform retrieval.

It only performs:

1. file-presence checks
2. graph-alignment checks
3. boundary checks
4. drift checks
5. status-legality checks

If `check` fails, Codex should then use Wendao SQL to retrieve only the
relevant `skeleton` or `body` content.

## 13. Minimal Implementation Requirements

Version one only requires:

1. a `markdown` table
2. at least these columns:
   - `path`
   - `surface`
   - `heading_path`
   - `title`
   - `level`
   - `skeleton`
   - `body`
3. query support for markdown under `blueprint/` and `plan/`
4. a Codex read path that prefers `skeleton` before `body`

## 14. Rejections

This RFC explicitly rejects:

1. adding `[query]` to `qianji.toml`
2. adding `[[query.lane]]` to `qianji.toml`
3. adding new retrieval syntax to the `qianji` CLI
4. treating `tree` as retrieval
5. treating `skeleton` as summary

## 15. Conclusion

Version one retrieval should stay simple:

1. structure display: `qianji show`
2. legality checks: `qianji check`
3. exact retrieval: `wendao sql`
4. default content surface: `markdown.skeleton`
5. on-demand content surface: `markdown.body`

This is enough to support the bounded Codex loop without introducing a new
DSL layer.
