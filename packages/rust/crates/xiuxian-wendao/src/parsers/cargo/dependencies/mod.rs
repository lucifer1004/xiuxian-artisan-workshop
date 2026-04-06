mod api;
mod regex;
mod types;

pub use self::api::parse_cargo_dependencies;
pub use self::types::CargoDependency;

#[cfg(test)]
#[path = "../../../../tests/unit/parsers/cargo/dependencies.rs"]
mod tests;
