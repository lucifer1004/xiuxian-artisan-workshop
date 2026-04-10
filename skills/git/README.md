---
type: knowledge
metadata:
  title: "Git Skill - Procedural Knowledge"
---

# Git Skill - Procedural Knowledge

## Overview

This skill provides basic git operations plus staging safeguards for clean, safe commits.

## Architecture

```
skills/git/
├── SKILL.md              # Skill manifest + LLM context
├── README.md             # This file
├── scripts/              # Git helpers used by the retained runtime surface
│   ├── commit.py         # Commit operations
│   ├── prepare.py        # Staging + security scan helper
│   ├── rendering.py      # Template rendering helper
│   └── ...
├── templates/            # Cascading templates for commit output
│   ├── commit_message.j2
│   └── error_message.j2
└── tests/                # Zero-config pytest
    └── test_git_status.py
```

---

## Staging Safeguards

Use the staging helper before committing when you want security scanning and
pre-commit hook execution:

1. Stage all tracked and untracked changes.
2. Run the project pre-commit hook when available.
3. Re-stage files modified by formatters.
4. Unstage obvious sensitive files.

Typical sequence:

```bash
tool: `git.stage_all`
tool: `git.commit` with `{"message": "type(scope): description"}`
```

---

## Security Guard Detection

The commit workflow includes **automated security scanning**:

### Sensitive File Patterns

Detects and warns about:

```
*.env*       .env files (may contain secrets)
*.pem        Private keys
*.key        API keys
*.secret     Secret files
*.credentials*  Credential files
*.priv       Private keys
id_rsa*      SSH keys
id_ed25519*  SSH keys
```

### LLM Advisory

When sensitive files are detected, the LLM receives this guidance:

```
⚠️ Security Check

Detected X potentially sensitive file(s):
  ⚠️ .env.production

LLM Advisory: Please verify these files are safe to commit.
- Are they intentional additions (not accidentally staged)?
- Do they contain secrets, keys, or credentials?
- Should they be in .gitignore?

If unsure, press No and run git reset <file> to unstage.
```

---

## Available Commands

### Tool Runtime Calls

| Command              | Category | Description                   |
| -------------------- | -------- | ----------------------------- |
| `git.commit`         | write    | Execute commit with template  |
| `git.stage_all`      | write    | Stage all changes             |
| `git.status`         | read     | Get git status                |
| `git.branch`         | read     | List branches                 |
| `git.log`            | read     | Show recent commits           |
| `git.diff`           | read     | Show changes                  |
| `git.add`            | write    | Stage specific files          |

---

## Tools Available (No Tool Needed)

| Operation | Command      | Notes     |
| --------- | ------------ | --------- |
| Status    | `git status` | Read-only |
| Diff      | `git diff`   | Read-only |
| Branch    | `git branch` | Read-only |
| Log       | `git log`    | Read-only |

---

## File Locations

| Path                                          | Purpose                 |
| --------------------------------------------- | ----------------------- |
| `skills/git/scripts/commit.py`                | Commit commands         |
| `skills/git/scripts/prepare.py`               | Staging helper          |
| `skills/git/templates/`                       | Default templates       |
| `assets/templates/git/`                       | User override templates |
| `cog.toml`                                    | Scope configuration     |

---

## Related

- [Skills Documentation](../../docs/skills.md)
- [Trinity Architecture](../../docs/explanation/trinity-architecture.md)
- [ODF-EP Protocol](../../docs/reference/odf-ep-protocol.md)
