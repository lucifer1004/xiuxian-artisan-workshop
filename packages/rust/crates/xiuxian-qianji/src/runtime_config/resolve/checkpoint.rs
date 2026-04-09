use crate::runtime_config::env_vars::env_var_or_override;
use crate::runtime_config::model::{QianjiRuntimeCheckpointConfig, QianjiRuntimeEnv};
use crate::runtime_config::toml_config::QianjiTomlCheckpoint;
use xiuxian_macros::string_first_non_empty;

const DEFAULT_CHECKPOINT_VALKEY_URL: &str = "redis://127.0.0.1:6379/0";

pub(super) fn resolve_qianji_runtime_checkpoint(
    file_checkpoint: &QianjiTomlCheckpoint,
    runtime_env: &QianjiRuntimeEnv,
) -> QianjiRuntimeCheckpointConfig {
    let valkey_url = string_first_non_empty!(
        runtime_env.qianji_checkpoint_valkey_url.as_deref(),
        file_checkpoint.valkey_url.as_deref(),
        env_var_or_override(runtime_env, "QIANJI_VALKEY_URL").as_deref(),
        env_var_or_override(runtime_env, "VALKEY_URL").as_deref(),
        env_var_or_override(runtime_env, "REDIS_URL").as_deref(),
        Some(DEFAULT_CHECKPOINT_VALKEY_URL),
    );

    QianjiRuntimeCheckpointConfig { valkey_url }
}
