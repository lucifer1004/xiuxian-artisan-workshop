# Execution Plan Policy

This repository uses **ExecPlans** for work that is large, uncertain, or cross-cutting.

## When an ExecPlan Is Required

Create or update an ExecPlan when any of these apply:

1. The task spans multiple crates/packages or subsystems.
2. The task is expected to take multiple implementation steps with checkpoints.
3. The task has architectural ambiguity or non-trivial tradeoffs.
4. The task is risky (regression risk, migration risk, or production-facing behavior change).

Small, isolated fixes do not require a full ExecPlan.

## Where Plans Live

1. Policy file: `.agent/PLANS.md` (this file).
2. Active plans: `.agent/execplans/<slug>.md`.
3. One plan file per initiative.

## Required Plan Structure

Each plan file should contain these sections:

1. `# Title`
2. `## Purpose / Big Picture`
3. `## Progress`
4. `## Surprises & Discoveries`
5. `## Decision Log`
6. `## Outcomes & Retrospective`
7. `## Context and Orientation`
8. `## Plan of Work`
9. `## Concrete Steps`
10. `## Validation and Acceptance`
11. `## Idempotence and Recovery`
12. `## Artifacts and Notes`
13. `## Interfaces and Dependencies`
14. `## Change Log`

## Authoring Rules

1. Keep the plan self-contained so a new contributor can execute it without prior context.
2. Update `## Progress`, `## Decision Log`, and `## Change Log` as work advances.
3. Prefer concrete checkpoints over vague statements.
4. Include exact verification commands in `## Validation and Acceptance`.
5. Record rollback/retry behavior in `## Idempotence and Recovery`.

## Quick Start

1. Copy `.agent/execplans/_template.md` to a new slug file.
2. Fill `Purpose`, `Context`, and `Plan of Work` before coding.
3. Keep the plan current until the initiative is complete.
