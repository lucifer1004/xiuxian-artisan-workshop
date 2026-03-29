---
type: knowledge
title: "Xiuxian OS Testing Guide"
category: "developer"
tags:
  - developer
  - testing
saliency_base: 6.3
decay_rate: 0.04
metadata:
  title: "Xiuxian OS Testing Guide"
---

# Xiuxian OS Testing Guide

## Current Python Scope

Retained Python testing covers only:

1. `packages/python/foundation/tests`
2. `packages/python/core/tests`
3. `packages/python/xiuxian-wendao-py/tests`
4. focused `packages/python/test-kit/tests` helpers that still validate the
   retained adapter surface

Python agent/skill/runtime test suites are gone with the deleted packages.

## Recommended Commands

```bash
# Retained Python package tests
just test-python

# Direct package-level runs
uv run pytest packages/python/foundation/tests
uv run pytest packages/python/core/tests
uv run pytest packages/python/xiuxian-wendao-py/tests

# Rust validation
cargo check --workspace --all-targets
cargo nextest run --workspace
```

## Architecture Rule

When a change touches Rust-owned runtime behavior, validate that behavior in
Rust first. Python tests should cover only retained consumer/helper boundaries,
not deleted local runtime systems.
