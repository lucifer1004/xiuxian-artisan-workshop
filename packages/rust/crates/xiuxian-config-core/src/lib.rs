//! Unified cascading configuration kernel and local path helpers.
//!
//! ## Macros
//!
//! - `crate_resources_dir!` - Expands to an embedded `include_dir::Dir`
//!   rooted at the absolute `resources/` directory for the crate that invokes
//!   the macro.
//! - `toml_first_env!` - Resolves a TOML-owned value first, then falls back to
//!   a precedence-ordered env lookup chain.
//! - `first_some!` - Resolves the first `Some(...)` candidate from an ordered
//!   precedence chain.

xiuxian_testing::crate_test_policy_source_harness!("../tests/unit/lib_policy.rs");

pub use xiuxian_macros::crate_resources_dir;

/// Resolve the first `Some(...)` candidate from an ordered precedence chain.
#[macro_export]
macro_rules! first_some {
    ($candidate:expr $(,)?) => {
        $candidate
    };
    ($first:expr, $($rest:expr),+ $(,)?) => {{
        let resolved = $first;
        if resolved.is_some() {
            resolved
        } else {
            $crate::first_some!($($rest),+)
        }
    }};
}

/// Resolve a TOML-owned setting first and then fall back to a precedence-ordered
/// env lookup chain.
///
/// The first form returns a trimmed string:
/// `toml_first_env!(settings, "path.to.key", lookup, ["ENV_A", "ENV_B"], get_setting)`
///
/// The second form additionally applies a parser closure and falls back to env
/// when the TOML value is blank or fails to parse:
/// `toml_first_env!(settings, "path.to.key", lookup, ["ENV"], get_setting, parse)`
#[macro_export]
macro_rules! toml_first_env {
    ($settings:expr, $setting_key:expr, $lookup:expr, [$($env:expr),+ $(,)?], $get_setting:path) => {{
        $crate::toml_first_env_string(
            $get_setting($settings, $setting_key),
            $lookup,
            &[$($env),+],
        )
    }};
    ($settings:expr, $setting_key:expr, $lookup:expr, [$($env:expr),+ $(,)?], $get_setting:path, $parse:expr) => {{
        $crate::toml_first_env_parsed(
            $get_setting($settings, $setting_key),
            $lookup,
            &[$($env),+],
            $parse,
        )
    }};
}

mod error;
mod paths;
mod resolve;
mod spec;
mod test_support;

pub use error::ConfigCoreError;
pub use paths::{
    absolutize_path, normalize_config_home, resolve_cache_home, resolve_cache_home_from_value,
    resolve_config_home, resolve_data_home, resolve_path_from_value, resolve_project_root,
    resolve_project_root_or_cwd, resolve_project_root_or_cwd_from_value,
};
pub use resolve::{
    first_non_empty_lookup, first_non_empty_named_lookup, load_toml_value_with_imports,
    load_toml_value_with_imports_and_paths, lookup_bool_flag, lookup_parsed,
    lookup_positive_parsed, merge_toml_values, parse_bool_flag, parse_positive, parse_trimmed,
    resolve_and_load, resolve_and_load_with_paths, resolve_and_merge_toml,
    resolve_and_merge_toml_with_paths, toml_first_env_parsed, toml_first_env_string,
    toml_first_named_string, trimmed_non_empty,
};
pub use spec::{ArrayMergeStrategy, ConfigCascadeSpec};
pub use test_support::resolve_home_from_value;
