# Enable Repo ExecPlan Mode

## Purpose / Big Picture

Enable repository-level planning mode so complex tasks consistently use a shared, auditable execution-plan workflow.

## Progress

- [x] Define ExecPlan trigger rules in `AGENTS.md`.
- [x] Add central policy document at `.agent/PLANS.md`.
- [x] Add reusable template at `.agent/execplans/_template.md`.
- [ ] Apply template to the next major initiative plan.

## Surprises & Discoveries

- Existing repository guidance did not define a canonical plan location.

## Decision Log

- Decision: use `.agent/PLANS.md` as the single policy source and `.agent/execplans/` for active plans.
  - Rationale: explicit convention, easy discovery, low friction for contributors and agents.
  - Date: 2026-03-05.

## Outcomes & Retrospective

Pending completion after at least one production initiative runs end-to-end with this workflow.

## Context and Orientation

- `AGENTS.md` governs contributor and agent behavior.
- `.agent/PLANS.md` defines plan format and lifecycle expectations.
- `.agent/execplans/*.md` stores initiative-specific plans.

## Plan of Work

1. Add policy and trigger rules.
2. Add template and bootstrap plan artifact.
3. Validate discoverability and handoff usability.

## Concrete Steps

1. Update `AGENTS.md` with ExecPlan section and policy pointer.
2. Create `.agent/PLANS.md` with required sections and usage rules.
3. Create `.agent/execplans/_template.md`.
4. Create this bootstrap plan file.

## Validation and Acceptance

1. Confirm paths exist:
   - `.agent/PLANS.md`
   - `.agent/execplans/_template.md`
   - `.agent/execplans/enable-execplan-mode.md`
2. Confirm `AGENTS.md` contains ExecPlan policy pointer.

## Idempotence and Recovery

- Safe to re-run by editing or replacing plan files.
- No schema migration required.
- Rollback is a plain-file revert.

## Artifacts and Notes

- New plan policy and template files under `.agent/`.

## Interfaces and Dependencies

- Internal contributor workflow only.
- No runtime API or binary behavior change.

## Change Log

- 2026-03-05: initial plan-mode bootstrap applied.
