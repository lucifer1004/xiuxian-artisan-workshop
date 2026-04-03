use std::error::Error;
use std::path::Path;

use serde_yaml::Value;
use xiuxian_config_core::load_toml_value_with_imports;

pub(crate) fn load_test_settings_from_path(config_path: &Path) -> Result<Value, Box<dyn Error>> {
    let merged = load_toml_value_with_imports(config_path)?;
    let json = serde_json::to_string(&merged)?;
    Ok(serde_json::from_str(&json)?)
}
