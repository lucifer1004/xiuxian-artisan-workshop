use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

static LINK_GRAPH_CONFIG_HOME_OVERRIDE: OnceLock<RwLock<Option<String>>> = OnceLock::new();
static LINK_GRAPH_WENDAO_CONFIG_OVERRIDE: OnceLock<RwLock<Option<String>>> = OnceLock::new();

fn config_home_override_store() -> &'static RwLock<Option<String>> {
    LINK_GRAPH_CONFIG_HOME_OVERRIDE.get_or_init(|| RwLock::new(None))
}

fn wendao_config_override_store() -> &'static RwLock<Option<String>> {
    LINK_GRAPH_WENDAO_CONFIG_OVERRIDE.get_or_init(|| RwLock::new(None))
}

/// Override the global Wendao configuration home directory.
pub fn set_link_graph_config_home_override(path: &str) {
    let mut guard = match config_home_override_store().write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = Some(path.trim().to_string());
}

/// Override the global Wendao configuration file path.
pub fn set_link_graph_wendao_config_override(path: &str) {
    let mut guard = match wendao_config_override_store().write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = Some(path.trim().to_string());
}

/// Return the current Wendao config file override, if one exists.
#[must_use]
pub fn wendao_config_file_override() -> Option<PathBuf> {
    let guard = match wendao_config_override_store().read() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    guard.clone().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::{set_link_graph_wendao_config_override, wendao_config_file_override};

    #[test]
    fn override_store_round_trips_config_path() {
        set_link_graph_wendao_config_override("/tmp/wendao.toml");
        assert_eq!(
            wendao_config_file_override()
                .as_deref()
                .map(std::path::Path::to_string_lossy)
                .map(|value| value.to_string()),
            Some("/tmp/wendao.toml".to_string())
        );
    }
}
