# The Knowledge Fortress: Global Quality Standards (V4.0)

## 1. Blueprint-Driven Evolution (The First Law)

- **Standard**: No implementation shall occur without an approved **Draft Blueprint** in `.data/blueprints/`.
- **Audit Rule**: Cross-reference the physical `.rs` code against the logical intent in the linked blueprint.

## 2. Hyper-Modularity & Namespace Sovereignty

- **Standard**: Logic must be surgically split into domain-specific modules.
- **Audit Rule**: Reject any file exceeding 300 lines.

## 3. Tiered Verification Protocol (The Alchemical Gates)

To preserve development momentum, the Auditor MUST categorize verification into three tiers:

- **TIER-1 (Pulse)**: `fmt` / `format`.
  - **Trigger**: Continuous.
  - **Auditor Stance**: Silent background requirement.
- **TIER-2 (Heartbeat)**: `cargo check` / `pyright`.
  - **Trigger**: Active coding phase / Sub-module completion.
  - **Auditor Stance**: **Primary Command**. Use this to verify type-safety without overhead.
- **TIER-3 (Gate)**: `clippy` / `too_many_lines` / `test`.
  - **Trigger**: ONLY when a task is ready for [DONE].
  - **Auditor Stance**: **High-Energy Audit**. Forbidden to demand this during active TIER-2 coding.

## 4. Performance & Memory (Zero-Copy)

- **Standard**: Zero-copy via `Arc<str>` or `SharedString`.
- **Audit Rule**: Flag any `String::clone()` in hot code paths.

## 5. Dashboard Implementation Protocol

### 5.1 Alchemical Implementation Plan (AIP)

Before implementation, output an AIP Dashboard.

### 5.2 Artisan Audit Verdict (AAV)

After implementation, output an AAV Dashboard based on the Tiered Protocol above.
