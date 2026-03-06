use super::loader::load_qianji_toml;
use super::model::{QianjiRuntimeEnv, QianjiRuntimeLlmConfig, QianjiRuntimeWendaoIngesterConfig};
use super::pathing::{resolve_prj_config_home, resolve_project_root};
use super::toml_config::QianjiToml;
use std::io;

mod llm;
mod wendao;

/// Resolve `qianji.toml` and environment into an effective LLM runtime config.
///
/// # Errors
///
/// Returns [`io::Error`] when a discovered `qianji.toml` file cannot be read or parsed.
pub fn resolve_qianji_runtime_llm_config() -> io::Result<QianjiRuntimeLlmConfig> {
    resolve_qianji_runtime_llm_config_with_env(&QianjiRuntimeEnv::default())
}

/// Resolve config with explicit runtime environment overrides (for tests and tooling).
///
/// # Errors
///
/// Returns [`io::Error`] when a discovered `qianji.toml` file cannot be read or parsed.
pub fn resolve_qianji_runtime_llm_config_with_env(
    runtime_env: &QianjiRuntimeEnv,
) -> io::Result<QianjiRuntimeLlmConfig> {
    let file_cfg = load_runtime_file_cfg(runtime_env)?;
    llm::resolve_qianji_runtime_llm(&file_cfg.llm, runtime_env)
}

/// Resolve `qianji.toml` and environment into native `Wendao` ingestion defaults.
///
/// # Errors
///
/// Returns [`io::Error`] when a discovered `qianji.toml` file cannot be read or parsed.
pub fn resolve_qianji_runtime_wendao_ingester_config()
-> io::Result<QianjiRuntimeWendaoIngesterConfig> {
    resolve_qianji_runtime_wendao_ingester_config_with_env(&QianjiRuntimeEnv::default())
}

/// Resolve `Wendao` ingestion defaults with explicit runtime environment overrides.
///
/// # Errors
///
/// Returns [`io::Error`] when a discovered `qianji.toml` file cannot be read or parsed.
pub fn resolve_qianji_runtime_wendao_ingester_config_with_env(
    runtime_env: &QianjiRuntimeEnv,
) -> io::Result<QianjiRuntimeWendaoIngesterConfig> {
    let file_cfg = load_runtime_file_cfg(runtime_env)?;
    Ok(wendao::resolve_qianji_runtime_wendao_ingester(
        &file_cfg.memory_promotion.wendao,
        runtime_env,
    ))
}

fn load_runtime_file_cfg(runtime_env: &QianjiRuntimeEnv) -> io::Result<QianjiToml> {
    let project_root = resolve_project_root(runtime_env);
    let config_home = resolve_prj_config_home(runtime_env, &project_root);
    load_qianji_toml(runtime_env, &project_root, &config_home)
}
