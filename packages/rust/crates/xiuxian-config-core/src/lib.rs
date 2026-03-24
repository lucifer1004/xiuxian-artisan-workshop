//! Unified cascading configuration kernel and local path helpers.
//!
//! ## Macros
//!
//! - `crate_resources_dir!` - Expands to an embedded `include_dir::Dir`
//!   rooted at the absolute `resources/` directory for the crate that invokes
//!   the macro.

pub use xiuxian_macros::crate_resources_dir;

mod error;
mod paths;
mod resolve;
mod spec;
mod test_support;

pub use error::ConfigCoreError;
pub use paths::{
    absolutize_path, normalize_config_home, resolve_cache_home, resolve_config_home,
    resolve_data_home, resolve_project_root, resolve_project_root_or_cwd,
};
pub use resolve::{
    load_toml_value_with_imports, resolve_and_load, resolve_and_load_with_paths,
    resolve_and_merge_toml, resolve_and_merge_toml_with_paths,
};
pub use spec::{ArrayMergeStrategy, ConfigCascadeSpec};
pub use test_support::resolve_home_from_value;
