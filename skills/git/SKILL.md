---
type: skill
title: "Git Version Control Operations"
category: "workflows"
tags:
  - git
  - version-control
  - commit
name: git
description: Use when committing code, managing branches, pushing to remote, creating pull requests, or performing version control operations. Conforms to docs/reference/skill-routing-value-standard.md.
metadata:
  author: xiuxian-artisan-workshop
  version: "2.0.0"
  source: "https://github.com/tao3k/xiuxian-artisan-workshop/tree/main/assets/skills/git"
  routing_keywords:
    - "git"
    - "commit"
    - "push"
    - "pull"
    - "merge"
    - "rebase"
    - "checkout"
    - "stash"
    - "tag"
    - "commit code"
    - "save changes"
    - "commit changes"
    - "push code"
    - "save work"
    - "check in"
    - "submit code"
    - "version control"
    - "branch"
    - "repo"
    - "repository"
    - "history"
    - "diff"
    - "status"
    - "log"
    - "pr"
    - "pull request"
    - "code review"
  intents:
    - "Create pull request"
    - "Manage branches"
    - "Commit code"
    - "Stash changes"
    - "Merge branches"
    - "Rebase branches"
    - "Tag commits"
    - "Check git status"
---

# Git Skill

> **Code is Mechanism, Prompt is Policy**

`git` is the canonical query phrase for this skill.

## Architecture

This skill keeps its runnable command functions in `scripts/*.py`.
Commands are exposed through the retained tool runtime as `git.command_name`.

## Available Commands

| Command            | Description                                             |
| ------------------ | ------------------------------------------------------- |
| `git.status`       | Show working tree status                                |
| `git.stage_all`    | Stage all changes (with security scan)                  |
| `git.commit`       | Commit staged changes                                   |
| `git.push`         | Push to remote                                          |
| `git.log`          | Show commit logs                                        |

## Staged Files Feature

### Stage and Scan Workflow

The `stage_and_scan` function provides automatic staging with security validation:

```
Stage All Files → Security Scan → Lefthook Pre-commit → Finalize
```

#### Key Features

1. **Automatic Staging**

   ```python
   stage_and_scan(project_root=".")
   # Returns: {staged_files, diff, security_issues, lefthook_error}
   ```

2. **Security Scanning**
   - Detects sensitive files (`.env*`, `*.pem`, `*.key`, `*.secret`, etc.)
   - Automatically un-stages detected files
   - Returns list of security issues

3. **Lefthook Integration**
   - Runs pre-commit hooks after staging
   - Re-stages files modified by lefthook formatters
   - Returns lefthook output for review

### Staged Files Commands

| Command           | Description                               |
| ----------------- | ----------------------------------------- |
| `git.stage_all()` | Stage all changes with security scan      |
| `git.status()`    | Show staged files and working tree status |
| `git.diff()`      | Show staged diff                          |

### Security Patterns Detected

```
.env*, *.env*, *.pem, *.key, *.secret, *.credentials*
id_rsa*, id_ed25519*, *.priv
secrets.yml, secrets.yaml, credentials.yml
```

## Usage Guidelines

### Read Operations (Safe - Use Claude-native bash)

```bash
git status
git diff --cached
git diff
git log --oneline
```

### Write Operations (Use Tool Runtime Calls)

| Operation    | Tool                                  |
| ------------ | ------------------------------------- |
| Stage all    | `git.stage_all()` (scans for secrets) |
| Commit       | `git.commit(message="...")`           |
| Push         | `git.push()`                          |

## Key Principle

> **Read = Claude-native bash. Write = tool runtime calls.**
