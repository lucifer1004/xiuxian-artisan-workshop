//! `DeepSeek` cache-key invariants at crate-level test boundary.

use xiuxian_llm::test_support::{DeepseekCacheKeyInput, build_deepseek_cache_key_for_tests};

const MODEL_ROOT: &str = "model-root";
const PROMPT_A: &str = "prompt-a";
const PROMPT_B: &str = "prompt-b";
const ORIGINAL: &[u8] = &[1_u8, 2_u8, 3_u8];

#[test]
fn cache_key_changes_when_decode_budget_changes() {
    let key_a = build_deepseek_cache_key_for_tests(&DeepseekCacheKeyInput {
        model_root: MODEL_ROOT,
        prompt: PROMPT_A,
        base_size: 1024,
        image_size: 640,
        crop_mode: true,
        max_new_tokens: 256,
        original: ORIGINAL,
    });
    let key_b = build_deepseek_cache_key_for_tests(&DeepseekCacheKeyInput {
        model_root: MODEL_ROOT,
        prompt: PROMPT_A,
        base_size: 1024,
        image_size: 640,
        crop_mode: true,
        max_new_tokens: 512,
        original: ORIGINAL,
    });

    assert_ne!(key_a, key_b);
}

#[test]
fn cache_key_changes_when_prompt_or_vision_changes() {
    let baseline = build_deepseek_cache_key_for_tests(&DeepseekCacheKeyInput {
        model_root: MODEL_ROOT,
        prompt: PROMPT_A,
        base_size: 1024,
        image_size: 640,
        crop_mode: true,
        max_new_tokens: 256,
        original: ORIGINAL,
    });
    let prompt_changed = build_deepseek_cache_key_for_tests(&DeepseekCacheKeyInput {
        model_root: MODEL_ROOT,
        prompt: PROMPT_B,
        base_size: 1024,
        image_size: 640,
        crop_mode: true,
        max_new_tokens: 256,
        original: ORIGINAL,
    });
    let vision_changed = build_deepseek_cache_key_for_tests(&DeepseekCacheKeyInput {
        model_root: MODEL_ROOT,
        prompt: PROMPT_A,
        base_size: 768,
        image_size: 768,
        crop_mode: true,
        max_new_tokens: 256,
        original: ORIGINAL,
    });

    assert_ne!(baseline, prompt_changed);
    assert_ne!(baseline, vision_changed);
}
