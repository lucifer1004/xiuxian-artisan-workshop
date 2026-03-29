---
type: knowledge
metadata:
  title: "Documentation Workflow"
---

# Documentation Workflow

> **TL;DR**: When code changes, docs MUST be updated. Use the Documentation Skill to manage knowledge entries.

---

## Quick Reference

| Task                    | Tool/Command                                    |
| ----------------------- | ----------------------------------------------- |
| Create knowledge entry  | add/update docs under `assets/knowledge/`       |
| Rebuild knowledge index | rerun the owning package/index flow if needed   |
| Search knowledge base   | use the retained knowledge query surface        |

---

## 1. The Documentation Rule

> **Rule**: Feature code cannot be merged until documentation is updated.

| If you modify...       | You must update...                                         |
| ---------------------- | ---------------------------------------------------------- |
| `skills/*/scripts/*`   | Skill documentation in `skills/*/SKILL.md` and `README.md` |
| `assets/specs/*.md`    | The matching workflow/process docs                          |
| `assets/how-to/*.md`   | Update the how-to itself                                    |
| `docs/*.md`            | User-facing guides (if breaking changes)                   |
| `CLAUDE.md`            | Project conventions                                        |
| `justfile`             | Command documentation in `docs/`                           |

---

## 2. The Documentation Workflow

```
┌─────────────────────────────────────────────────────────────────┐
│  Code implementation complete                                   │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│  Step 1: Determine doc type                                     │
│  - New knowledge → Add a focused note under assets/knowledge/  │
│  - Code changes → Update relevant docs in docs/ or assets/     │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│  Step 2: Create or update documentation                         │
│  - Add or revise the relevant knowledge/how-to/doc entry       │
│  - Keep package docs and GTD in sync                           │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│  Step 3: Rebuild or resync retained indexes if required        │
│  (only where the owning package still keeps one)               │
└─────────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────────┐
│  Step 4: Commit with docs                                       │
│  just agent-commit                                              │
└─────────────────────────────────────────────────────────────────┘
```

---

## 3. Using the Documentation Skill

### Create a Knowledge Entry

```python
Create a focused markdown note under `assets/knowledge/` or update the
relevant existing guide in place.
```

**Response:**

```
Updated knowledge note: `assets/knowledge/...`
```

### Rebuild Knowledge Index

Run the owning package or documentation sync flow only if that area keeps a
derived index:

```python
direnv exec . <package-or-doc sync command>
```

### Search Knowledge Base

```python
Use the retained query surface or repository search to locate the note.
```

---

## 4. Knowledge Entry Standards

### Location

- Knowledge notes belong under `assets/knowledge/` unless a package-specific
  doc tree owns the content.

### Naming Convention

`YYYYMMDD-category-title.md` (e.g., `20260102-debugging-nested-locks.md`)

### Frontmatter Format

```markdown
# Title

> **Category**: CATEGORY | **Date**: YYYY-MM-DD

Content...
```

### Categories

- `architecture` - Design decisions
- `debugging` - Problem solutions
- `pattern` - Reusable patterns
- `workflow` - Process documentation
- `domain` - Domain-specific knowledge

---

## 5. Document Classification

Understand where to write documentation:

| Directory         | Audience     | Purpose                                            |
| ----------------- | ------------ | -------------------------------------------------- |
| `assets/how-to/` | Operators    | How-to guides and workflows                         |
| `docs/`          | Users        | Human-readable manuals, tutorials                   |
| `skills/*/`      | LLM + Devs   | Skill documentation (`SKILL.md`, `README.md`)       |
| `assets/specs/`  | LLM + Devs   | Feature specifications                              |

---

## 6. When to Write Documentation

| Scenario               | Write To                                                           |
| ---------------------- | ------------------------------------------------------------------ |
| New skill              | `skills/{skill}/SKILL.md` and `skills/{skill}/README.md`           |
| New workflow/process   | `assets/how-to/`                                                   |
| User-facing guide      | `docs/` (for humans)                                               |
| Implementation details | `skills/*/` (for contributors)                                     |
| Feature spec           | `assets/specs/` (contract between requirements and implementation) |
| Project convention     | `CLAUDE.md` (quick reference)                                      |
| Captured insight       | `assets/knowledge/`                                                |

---

## 7. Anti-Patterns

| Wrong                               | Correct                                                        |
| ----------------------------------- | -------------------------------------------------------------- |
| Commit code without updating README | Check relevant docs first                                      |
| Update docs in a separate commit    | Update docs in the SAME commit as code                         |
| Write user docs in internal how-to  | Write user docs in `docs/`                                     |
| Forget to update CLAUDE.md          | Update CLAUDE.md for new tools/commands                        |
| Store insights without sync         | Update the owning package/docs index only where one still exists |

---

## 8. Related Documentation

| Document                               | Purpose                          |
| -------------------------------------- | -------------------------------- |
| `assets/how-to/gitops.md`              | Commit conventions               |
| `assets/how-to/testing-workflows.md`   | Test requirements                |
| `assets/knowledge/`                    | Knowledge notes                  |

---

_Document everything. Code without docs is debt, not asset._
