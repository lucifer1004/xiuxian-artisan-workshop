---
type: knowledge
metadata:
  title: "Git Skill"
---

# Git Skill

## Overview

This directory no longer ships a local Python git runtime.

## Current Boundary

1. `skills/git` retains metadata and knowledge surfaces only.
2. Python scripts, local pytest modules, and runtime-coupled templates were
   retired from this directory.
3. Any future executable git integration must be owned outside `skills/`.

## Use

Use native git CLI commands or app-owned git integrations rather than expecting
deleted local script helpers to exist in this directory.

## Retained Artifacts

| Path                             | Purpose                         |
| -------------------------------- | ------------------------------- |
| `skills/git/SKILL.md`            | Metadata and routing context    |
| `skills/git/README.md`           | Boundary documentation          |
| `skills/git/assets/Backlog.md`   | Historical backlog notes        |
| `skills/git/extensions/sniffer/` | Non-Python retained rule assets |
