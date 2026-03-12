# Execution Plan Policy

This repository uses ExecPlans for work that is large, uncertain, or cross cutting.

## Plan First Gate (Mandatory)

Before any file reads, searches, or command execution (including tests), you must present a plan.

1. For small tasks, a Micro Plan inside the assistant response is acceptable.
2. For large or risky tasks, an ExecPlan file is required and must be created or updated before any further action.
3. If new files, commands, or scope are discovered, the plan must be updated and acknowledged again.
4. Plan Self Check must be completed before any file reads, searches, or commands.

## Plan Types

1. Micro Plan
   Details: For isolated, low risk changes (single file or trivial edits). Must list files to read and commands to run. Does not require a plan file.
2. ExecPlan
   Details: For multi step, cross crate or package, or risky work. Must be stored as a plan file and kept current.

## When an ExecPlan Is Required

1. The task spans multiple crates, packages, or subsystems.
2. The task is expected to take multiple implementation steps with checkpoints.
3. The task has architectural ambiguity or nontrivial tradeoffs.
4. The task is risky (regression risk, migration risk, or production facing behavior change).

Small, isolated fixes do not require a full ExecPlan.

## Where Plans Live

1. Policy file: `.agent/PLANS.md` (this file).
2. Template file: `.agent/execplans/_template.md`.
3. Active plans: `.cache/codex/execplans/<slug>.md`.

## Required Plan Structure

Each plan file should contain these sections:

1. `# Title`
2. `## Purpose / Big Picture`
3. `## Scope and Boundaries`
4. `## Plan Self Check`
5. `## Context and Orientation`
6. `## Plan of Work`
7. `## Concrete Steps`
8. `## Validation and Acceptance`
9. `## Reflection and Quality Audit`
10. `## Final Validation Gate`
11. `## Idempotence and Recovery`
12. `## Interfaces and Dependencies`
13. `## Progress`
14. `## Decision Log`
15. `## Surprises & Discoveries`
16. `## Artifacts and Notes`
17. `## Outcomes & Retrospective`
18. `## Change Log`

## Scope and Boundaries (Required Detail)

This section must include:

1. Files or dirs to read
2. Commands or tools to run (including tests)
3. Expected outputs
4. Stop conditions

Any activity outside this scope requires a plan update and reacknowledgement.

## Plan Self Check (Required)

This section must include:

1. Scope matches the request and risk level.
2. Files or dirs to read are complete and minimal.
3. Commands or tools to run are complete and safe.
4. Expected outputs are concrete and testable.
5. Stop conditions are clear.
6. Dependencies and constraints are recorded.
7. Validation plan is adequate for risk.
8. Plan type is correct.

Work must not proceed until this self check is complete.

## Authoring Rules

1. Keep the plan self contained so a new contributor can execute it without prior context.
2. Update `## Progress`, `## Decision Log`, and `## Change Log` as work advances.
3. Prefer concrete checkpoints over vague statements.
4. Include exact verification commands in `## Validation and Acceptance`.
5. Record rollback or retry behavior in `## Idempotence and Recovery`.

## Reflection and Quality Audit (Required)

This section must include:

1. Code audit: correctness, reliability, security, performance, and maintenance risk.
2. Plan audit: scope adherence, deviations, and any unexecuted steps.
3. Verification audit: what ran, what did not run, and why that is acceptable.

## Final Validation Gate

This section must be the last checkpoint before marking work DONE.

1. Confirm `## Validation and Acceptance` is complete.
2. Confirm `## Reflection and Quality Audit` is recorded.
3. State a final go or no go decision with rationale.

## Quick Start

1. Copy `.agent/execplans/_template.md` to `.cache/codex/execplans/<slug>.md`.
2. Fill `Purpose`, `Scope and Boundaries`, `Context`, and `Plan of Work` before coding.
3. Keep the plan current until the initiative is complete.
