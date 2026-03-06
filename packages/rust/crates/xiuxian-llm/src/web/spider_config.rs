use std::sync::OnceLock;

use serde::Deserialize;
use tracing::warn;

#[xiuxian_macros::xiuxian_config(
    namespace = "llm.web.spider",
    internal_path = "resources/config/web_spider.toml",
    orphan_file = ""
)]
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
struct SpiderTomlConfig {
    user_agent: Option<String>,
    chrome_intercept: Option<bool>,
    prefer_raw_html_on_clean_empty: Option<bool>,
}

static CONFIG: OnceLock<SpiderTomlConfig> = OnceLock::new();

fn config() -> &'static SpiderTomlConfig {
    CONFIG.get_or_init(load_config)
}

pub(super) fn user_agent() -> Option<String> {
    config()
        .user_agent
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(super) fn chrome_intercept() -> Option<bool> {
    config().chrome_intercept
}

pub(super) fn prefer_raw_html_on_clean_empty() -> Option<bool> {
    config().prefer_raw_html_on_clean_empty
}

fn load_config() -> SpiderTomlConfig {
    SpiderTomlConfig::load().unwrap_or_else(|error| {
        warn!(
            event = "llm.web.spider.config.load_failed",
            error = %error,
            "Spider config load failed, falling back to defaults"
        );
        SpiderTomlConfig::default()
    })
}
