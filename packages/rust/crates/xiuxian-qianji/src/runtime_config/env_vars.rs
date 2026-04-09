use super::model::QianjiRuntimeEnv;
use xiuxian_config_core::{parse_bool_flag, parse_positive, trimmed_non_empty};

pub(super) fn env_var_or_override(runtime_env: &QianjiRuntimeEnv, key: &str) -> Option<String> {
    match env_override_state(runtime_env, key) {
        EnvOverrideState::Value(value) => return Some(value),
        EnvOverrideState::Empty => return Some(String::new()),
        EnvOverrideState::Missing => {}
    }
    read_env_non_empty(key)
}

pub(super) fn parse_usize_env_override(runtime_env: &QianjiRuntimeEnv, key: &str) -> Option<usize> {
    env_var_or_override(runtime_env, key).and_then(|value| parse_positive::<usize>(&value))
}

pub(super) fn parse_bool_env_override(runtime_env: &QianjiRuntimeEnv, key: &str) -> Option<bool> {
    env_var_or_override(runtime_env, key).and_then(|value| parse_bool_flag(value.as_str()))
}

pub(super) fn normalize_non_empty(value: Option<String>) -> Option<String> {
    trimmed_non_empty(value)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum EnvOverrideState {
    Missing,
    Empty,
    Value(String),
}

fn env_override_state(runtime_env: &QianjiRuntimeEnv, key: &str) -> EnvOverrideState {
    let Some((_, value)) = runtime_env
        .extra_env
        .iter()
        .find(|(candidate_key, _)| candidate_key == key)
    else {
        return EnvOverrideState::Missing;
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        EnvOverrideState::Empty
    } else {
        EnvOverrideState::Value(trimmed.to_string())
    }
}

fn read_env_non_empty(key: &str) -> Option<String> {
    xiuxian_macros::env_non_empty!(key)
}

#[cfg(test)]
#[path = "../../tests/unit/runtime_config/env_vars.rs"]
mod tests;
