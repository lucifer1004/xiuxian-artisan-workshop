//! Unified cascading configuration kernel.

mod cache;
mod error;
mod paths;
mod resolve;
mod spec;
#[doc(hidden)]
pub mod test_support;

pub use error::ConfigCoreError;
pub use paths::{
    absolutize_path, normalize_config_home, resolve_cache_home, resolve_config_home,
    resolve_data_home, resolve_project_root, resolve_project_root_or_cwd,
};
pub use resolve::{
    resolve_and_load, resolve_and_load_with_paths, resolve_and_merge_toml,
    resolve_and_merge_toml_with_paths,
};
pub use spec::{ArrayMergeStrategy, ConfigCascadeSpec};
