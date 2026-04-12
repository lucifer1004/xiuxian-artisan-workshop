//! TOML configuration loading and persistence for Studio API.

mod load;
mod paths;
mod persist;
mod sanitize;
#[cfg(test)]
#[path = "../../../../../tests/unit/gateway/studio/router/config/mod.rs"]
mod tests;
mod types;

pub use load::{load_ui_config_from_wendao_toml, load_ui_config_from_wendao_toml_path};
pub use paths::{
    resolve_studio_config_root, studio_effective_wendao_toml_path, studio_wendao_overlay_toml_path,
    studio_wendao_toml_path,
};
pub use persist::{persist_ui_config_to_wendao_toml, persist_ui_config_to_wendao_toml_path};
