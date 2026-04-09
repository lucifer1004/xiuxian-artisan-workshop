//! Category inference for script scanners.
//!
//! Provides sensible default categories when not explicitly specified
//! in the @`skill_command` decorator.

mod infer;
mod rules;

pub use infer::infer_category_from_skill;

#[cfg(test)]
#[path = "../../../../tests/unit/skills/skill_command/category.rs"]
mod tests;
