---
type: knowledge
title: "Discovery"
category: "developer"
tags:
  - developer
  - discover
saliency_base: 6.3
decay_rate: 0.04
metadata:
  title: "Discovery"
---

# Discovery

## Status

The historical Python discovery stack has been removed.

Python no longer owns:

1. skill discovery
2. skill indexing
3. router discovery
4. local registry-based discovery helpers

Those responsibilities now live in Rust services.

## Developer Rule

Do not add new Python discovery layers. If a workflow needs discovery or
indexing behavior, it must consume Rust-owned contracts instead of recreating a
Python-local discovery stack.
