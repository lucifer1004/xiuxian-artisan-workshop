use tracing::warn;

use super::raw::DeepseekTomlConfig;

pub(super) fn load_config() -> DeepseekTomlConfig {
    DeepseekTomlConfig::load().unwrap_or_else(|error| {
        warn!(
            event = "llm.vision.deepseek.config.load_failed",
            error = %error,
            "DeepSeek config load failed, falling back to empty optional overrides"
        );
        DeepseekTomlConfig::default()
    })
}
