---
type: knowledge
title: "Hot Reload Mechanism"
category: "developer"
tags:
  - developer
  - hot
saliency_base: 6.3
decay_rate: 0.04
metadata:
  title: "Hot Reload Mechanism"
---

# Hot Reload Mechanism

## Status

The historical Python hot-reload stack has been removed. Any live hot-reload
behavior is Rust-owned.

## Practical Rule

When debugging stale state, inspect Rust runtime caches and service/Flight
transport state, then restart the relevant Rust-owned process if needed. Do
not rely on deleted Python hot-reload surfaces.
