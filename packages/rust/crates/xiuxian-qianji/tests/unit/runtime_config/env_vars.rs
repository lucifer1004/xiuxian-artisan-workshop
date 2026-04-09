use super::{
    env_var_or_override, normalize_non_empty, parse_bool_env_override, parse_usize_env_override,
};
use crate::runtime_config::model::QianjiRuntimeEnv;

#[test]
fn parse_usize_env_override_rejects_zero() {
    let runtime_env = QianjiRuntimeEnv {
        extra_env: vec![("LIMIT".to_string(), "0".to_string())],
        ..QianjiRuntimeEnv::default()
    };
    assert_eq!(parse_usize_env_override(&runtime_env, "LIMIT"), None);
}

#[test]
fn parse_bool_env_override_accepts_bool_aliases() {
    let runtime_env = QianjiRuntimeEnv {
        extra_env: vec![("ENABLED".to_string(), " yes ".to_string())],
        ..QianjiRuntimeEnv::default()
    };
    assert_eq!(parse_bool_env_override(&runtime_env, "ENABLED"), Some(true));
}

#[test]
fn normalize_non_empty_trims_values() {
    assert_eq!(
        normalize_non_empty(Some(" trimmed ".to_string())),
        Some("trimmed".to_string())
    );
}

#[test]
fn env_var_or_override_preserves_empty_override_as_blocking_value() {
    let runtime_env = QianjiRuntimeEnv {
        extra_env: vec![("QIANJI_CONFIG_PATH".to_string(), "   ".to_string())],
        ..QianjiRuntimeEnv::default()
    };
    assert_eq!(
        env_var_or_override(&runtime_env, "QIANJI_CONFIG_PATH"),
        Some(String::new())
    );
}
