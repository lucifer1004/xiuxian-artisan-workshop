# Xiuxian LLM DocOS Kernel: Map of Content

:PROPERTIES:
:ID: def9b49205caf472b4e687cadd0df348235357f1
:TYPE: INDEX
:STATUS: ACTIVE
:END:

Standardized documentation repository for `xiuxian-llm`, with the current emphasis on DeepSeek OCR runtime parity, Metal guard profiles, and model-side memory investigation.

Default feature topology keeps only `provider-litellm` enabled. `local-llm` is the opt-in
umbrella for local inference, with `mistral.rs` and the `vision-dots` DeepSeek OCR stack
classified under that local feature family. The effective child layout is
`local-llm = ["mistral.rs", "vision-dots"]`.

## 01_core: Architecture and Foundation

:PROPERTIES:
:ID: llm-core-foundation
:END:

- [01_core/101_vision_runtime_topology.md](01_core/101_vision_runtime_topology.md): Runtime topology from `xiuxian-llm` into vendored `deepseek-ocr`.
- [[01_core/101_xiuxian_llm_core_boundary]]

## 03_features: Functional Ledger

:PROPERTIES:
:ID: llm-functional-ledger
:END:

- [03_features/201_deepseek_ocr_runtime.md](03_features/201_deepseek_ocr_runtime.md): Effective runtime and loader behavior for DeepSeek OCR.
- [03_features/202_metal_guard_profiles.md](03_features/202_metal_guard_profiles.md): Guarded CPU and Metal profile conventions.
- [[03_features/201_xiuxian_llm_feature_ledger]]

## 05_research: Investigations and Findings

- [05_research/301_deepseek_ocr_metal_memory.md](05_research/301_deepseek_ocr_metal_memory.md): Current DeepSeek OCR Metal memory findings and validated hypotheses.
- [05_research/302_mistralrs_loading_patterns.md](05_research/302_mistralrs_loading_patterns.md): Which `mistralrs-core` loading and MoE patterns are worth porting before touching `candle-core`.
- [[05_research/301_xiuxian_llm_research_agenda]]

## 06_roadmap: Near-Term Evolution

- [06_roadmap/401_metal_stability.md](06_roadmap/401_metal_stability.md): Short-horizon plan for reaching stable guarded Metal inference.
- [[06_roadmap/401_xiuxian_llm_roadmap]]

:RELATIONS:
:LINKS: [[06_roadmap/401_xiuxian_llm_roadmap]], [[05_research/301_xiuxian_llm_research_agenda]], [[03_features/201_xiuxian_llm_feature_ledger]], [[01_core/101_xiuxian_llm_core_boundary]]
:END:

---

:FOOTER:
:STANDARDS: v2.0
:LAST_SYNC: 2026-03-24
:END:
