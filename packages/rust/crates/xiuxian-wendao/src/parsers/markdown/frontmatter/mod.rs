mod api;
mod types;

pub use self::api::parse_frontmatter;
pub use self::types::NoteFrontmatter;

#[cfg(test)]
#[path = "../../../../tests/unit/parsers/markdown/frontmatter.rs"]
mod tests;
