---
type: knowledge
title: "RFC 0005: Markdown Skeleton Minimal Rules for Bounded Plan Work"
category: "rfc"
status: "draft"
authors:
  - codex
created: 2026-04-08
tags:
  - rfc
  - qianji
  - wendao
  - markdown
  - skeleton
  - planning
---

# RFC 0005: Markdown Skeleton Minimal Rules for Bounded Plan Work

## 1. Summary

This RFC defines the minimal rules for markdown `skeleton` generation inside
bounded plan work surfaces.

Version one only needs to answer four questions:

1. what `skeleton` is
2. what `skeleton` must preserve
3. what `skeleton` may omit
4. how Codex should use `skeleton` before `body`

The goal is to keep the retrieval surface structurally faithful and low-token
at the same time.

## 2. Background

RFC 0003 already defines `markdown.skeleton` as the default retrieval surface
and `markdown.body` as the second-level retrieval surface.

RFC 0004 already defines the bounded work surface and the default
`qianji show --dir <plan-workdir>` plus
`qianji check --dir <plan-workdir>` loop.

What still needs to be fixed is the smallest stable answer to:

> when Wendao returns `skeleton`, what exactly should Codex expect to see?

Without that contract, `skeleton` can drift into either:

1. an underspecified summary surface
2. an almost-full-body surface that loses the token advantage

## 3. Non-Goals

This RFC does not define:

1. the full markdown table schema
2. AST-internal storage design
3. code-block-specific formatting rules
4. rich semantic extraction from prose
5. repository-wide documentation style rules

Version one only defines the external `skeleton` contract for bounded plan
work.

## 4. Structural Unit Assumption

`skeleton` is generated from a retrievable markdown structural unit, as
described in RFC 0003.

That unit may be:

1. a document root
2. a heading subtree
3. a section that can be rendered as one stable structural slice

This RFC does not force one internal segmentation strategy. It only fixes what
the resulting `skeleton` should look like.

## 5. What `skeleton` Is

`skeleton` is a structural compression view.

It is not:

1. a prose summary
2. a semantic paraphrase
3. a full-content dump

It is a compact rendering of the original markdown structure that keeps the
elements Codex needs for bounded planning work.

## 6. Required Preserved Elements

Version one `skeleton` must preserve these elements whenever they are present.

### 6.1 Heading Line

The current heading must be preserved.

### 6.2 Heading Hierarchy

Visible child-heading relationships must be preserved so Codex can still see
the document shape.

### 6.3 Key Property Fields

Key property-drawer fields or equivalent structured metadata must be preserved
when they affect planning or execution boundaries.

### 6.4 Checklist Shape

Checklist items must preserve their shape and completion state.

### 6.5 Key Task Outline

Short task bullets or execution-outline bullets should be preserved when they
express structure rather than long narrative explanation.

## 7. Elements That May Be Omitted or Compressed

Version one `skeleton` may omit or compress:

1. long explanatory paragraphs
2. repeated prose examples
3. fully expanded narrative details that do not change the structural shape
4. large bodies that are only needed for close reading

The intent is to lower token cost without destroying the planning shape.

## 8. What Must Not Happen

`skeleton` must not drift into these failure modes:

### 8.1 Summary Mode

It must not rewrite the source into a few explanatory sentences.

### 8.2 Full-Body Mode

It must not become a near-verbatim copy of `body` by default.

### 8.3 Shape Loss

It must not discard heading structure, checklist shape, or key boundary
metadata that Codex needs to reason about the plan.

## 9. Surface Symmetry

The same `skeleton` principles apply to both top-level bounded surfaces:

1. `blueprint/`
2. `plan/`

Version one should not make `blueprint` skeletons radically different from
`plan` skeletons. The content may differ, but the compression contract should
stay structurally consistent.

## 10. Codex Read Order

The recommended read order remains:

```text
1. qianji show --dir <plan-workdir>
2. inspect filenames and top-level directory shape
3. if needed, use tree only as a structural probe
4. read markdown.skeleton through wendao sql
5. read markdown.body only when skeleton is not enough
6. edit only inside <plan-workdir>
7. qianji check --dir <plan-workdir>
```

The key rule is:

> Codex should default to `skeleton` first and only escalate to `body` when
> structural compression is not enough for the current edit.

## 11. Minimal Examples

### 11.1 Heading and Checklist

Source:

```markdown
# Plan

## Rust

- [ ] audit boundary
- [x] confirm flowchart

Long explanatory prose...
```

A valid `skeleton` may look like:

```markdown
# Plan

## Rust

- [ ] audit boundary
- [x] confirm flowchart
```

### 11.2 Property-Like Metadata

Source:

```markdown
# Blueprint

:owner: codex
:status: active

## Boundary

Detailed narrative text...
```

A valid `skeleton` may look like:

```markdown
# Blueprint

:owner: codex
:status: active

## Boundary
```

## 12. Relation to Wendao SQL

This RFC does not add new retrieval syntax.

Codex should continue to retrieve `skeleton` through Wendao SQL, for example:

```bash
wendao sql "
select path, heading_path, skeleton
from markdown
where surface in ('blueprint', 'plan')
order by path, heading_path
"
```

If the current task still needs exact prose, Codex may then retrieve `body`.

## 13. Rejections

This RFC explicitly rejects:

1. treating `skeleton` as summary
2. treating `skeleton` as default full content
3. letting each document invent unrelated skeleton semantics
4. coupling `skeleton` generation to a new Qianji DSL

## 14. Conclusion

Version one `skeleton` should stay simple:

1. preserve structural shape
2. preserve key metadata
3. preserve checklist and task outline shape
4. compress long prose
5. remain smaller than `body`

That is enough to support bounded Codex reading without losing the plan's
structural meaning.
