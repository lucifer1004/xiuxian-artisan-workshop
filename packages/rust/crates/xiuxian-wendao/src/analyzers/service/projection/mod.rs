//! Repository projection functions (projected pages, retrieval, navigation, and gap reports).

mod family;
mod gap;
mod index_tree;
mod navigation;
mod page;
mod pages;
#[path = "planner/mod.rs"]
mod planner;
mod registry;
mod retrieval;
mod search;

#[cfg(test)]
#[path = "../../../../tests/unit/analyzers/service/projection/mod.rs"]
mod tests;

pub use family::*;
pub use gap::*;
pub use index_tree::*;
pub use navigation::*;
pub use page::*;
pub use pages::*;
pub use planner::*;
pub use retrieval::*;
pub use search::*;
