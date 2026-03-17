# Vision Runtime Topology

:PROPERTIES:
:ID: llm-vision-runtime-topology
:PARENT: [[../index.md]]
:STATUS: ACTIVE
:END:

## Runtime Layers

1. `xiuxian-llm` resolves configuration from TOML, environment overrides, and test profiles.
2. The native DeepSeek engine loader prepares model root, weights path, snapshot policy, device, and dtype.
3. Vendored `deepseek-ocr` loads the OCR model, vision backbones, and language model.
4. Guarded real-device tests exercise the same runtime through explicit profiles.

## Important Boundaries

### `xiuxian-llm`

- Owns config resolution, runtime lifecycle, guard profiles, and caller-facing execution.
- Must remain close to upstream loader/runtime behavior while parity work is ongoing.

### Vendored `deepseek-ocr`

- Owns model load policy, DeepSeek OCR inference, MoE execution, and CLIP/SAM behavior.
- Current optimization work is intentionally constrained to this layer.

## Active DeepSeek OCR Flow

1. `vision_deepseek.toml` resolves the effective test or runtime profile.
2. `scripts/run_real_metal_test.py` converts the profile into concrete env overrides.
3. `xiuxian-llm` loads the native engine with the requested device and guard settings.
4. `deepseek-ocr` performs image embedding, prompt materialization, language prefill, and decode.

## Why This Topology Matters

When a guarded Metal run fails, we must first determine whether the failure belongs to:

- config drift in `xiuxian-llm`
- lifecycle drift in `xiuxian-llm`
- model load or execution behavior in `deepseek-ocr`

Recent DeepSeek OCR work has already shown that several failures once attributed to wrapper behavior were actually caused by vendored model policy scope or MoE execution behavior.
