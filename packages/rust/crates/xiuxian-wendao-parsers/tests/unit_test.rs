//! Canonical unit test harness for `xiuxian-wendao-parsers`.

xiuxian_testing::crate_test_policy_harness!();

#[path = "unit/blocks.rs"]
mod blocks;
#[path = "unit/document.rs"]
mod document;
#[path = "unit/frontmatter.rs"]
mod frontmatter;
#[path = "unit/note.rs"]
mod note;
#[path = "unit/references.rs"]
mod references;
#[path = "unit/sections.rs"]
mod sections;
#[path = "unit/targets.rs"]
mod targets;
#[path = "unit/wikilinks.rs"]
mod wikilinks;
