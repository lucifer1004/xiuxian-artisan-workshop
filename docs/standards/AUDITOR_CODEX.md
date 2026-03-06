---
type: knowledge
title: "The Auditor's Codex: Engineering Standards & Quality Gates"
category: "standards"
tags:
  - auditing
  - performance
  - quality
  - rust
metadata:
  title: "The Auditor's Codex (V10.0 - High-Standard Era)"
---

# The Auditor's Codex (V10.0 - High-Standard Era)

This document defines the **Mandatory Engineering Standards** for the CyberXiuXian Workshop. All implementations must pass these gates.

## 1. High-Standard Code Quality (The Artisan's Guard)

- **Hyper-Modularity**: Logic must be split into fine-grained modules. No file shall exceed 300 lines without a modularity review.
- **Namespace Sovereignty**: Every symbol and constant must reside in its specific domain. Zero "misc" or "util" buckets.
- **Test Isolation**:
  - Unit tests MUST reside in `mod tests` or a dedicated `tests/` directory.
  - Integration tests MUST NOT pollute the `src/` directory.
  - Standard: "One Logic, One Test File."

## 2. Safety & Physical Integrity

- [SKILL-ANCHOR]: `SKILL.md` is the only physical blocker for discovery.
- [SCOPE-VIGILANCE]: Any file outside the `skills.toml` authorized set triggers a warning.
- [ZERO-LEAKAGE]: System-level errors must be scrubbed by `ZhenfaTransmuter`.
- [PROTOCOL-INTEGRITY]: All tool/function responses MUST be preceded by an Assistant request with a matching `tool_call_id`. The system MUST enforce a `Hygiene` layer (`enforce_tool_message_integrity`) before LLM dispatch.

## 3. Performance & Memory

- [ZERO-COPY]: Mandatory `Arc<str>` for resource sharing.
- [PARALLEL]: Mandatory `rayon` for all traversals.

## 5. Operational Efficiency & Tiered Gates

To preserve the Sovereign's development momentum, verification is divided into three power tiers:

- **[TIER-1: PULSE]**: `fmt`. Run on every file save. Purpose: Visual consistency.
- **[TIER-2: HEARTBEAT]**: `cargo check`. Run during active coding. Purpose: Type safety and syntax.
- **[TIER-3: GATE]**: `cargo clippy`, `cargo test`, `too_many_lines`. Run ONLY when a sub-task is ready for promotion to [DONE]. Purpose: Architectural alignment and performance audit.

**The Auditor shall only demand TIER-3 compliance at the point of Finality.**
