use litellm_rs::core::providers::base::BaseConfig;
use litellm_rs::core::providers::openai::config::{OpenAIConfig, OpenAIFeatures};
use std::collections::HashMap;

pub(super) fn build_openai_config(
    api_base: String,
    api_key: Option<String>,
    timeout_secs: u64,
) -> OpenAIConfig {
    OpenAIConfig {
        base: BaseConfig {
            api_key,
            api_base: Some(api_base),
            timeout: timeout_secs,
            max_retries: 3,
            headers: HashMap::default(),
            organization: None,
            api_version: None,
        },
        organization: None,
        project: None,
        model_mappings: HashMap::default(),
        features: OpenAIFeatures::default(),
    }
}
