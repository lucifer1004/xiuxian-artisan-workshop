## Vendored DeepSeek OCR Crates

This directory contains a minimal vendored workspace copied from
`https://github.com/TimmyOVO/deepseek-ocr.rs` at revision
`02b933df24f5658d10b37dd48c9c354d95c530c3`.

Only the crates required for `xiuxian-llm` OCR runtime patching are included:

- `deepseek-ocr-core`
- `deepseek-ocr-dsq`
- `deepseek-ocr-dsq-runtime`
- `deepseek-ocr-infer-deepseek`

The default behavior must remain upstream-compatible unless explicitly noted by
tests and configuration.
