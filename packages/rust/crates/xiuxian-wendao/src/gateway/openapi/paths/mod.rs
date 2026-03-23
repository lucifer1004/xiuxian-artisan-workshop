//! Route inventory shared by the Wendao gateway runtime and `OpenAPI` contract tests.

mod analysis;
mod docs;
mod graph;
mod repo;
mod search;
mod shared;
mod ui;
mod vfs;

pub use self::{analysis::*, docs::*, graph::*, repo::*, search::*, shared::*, ui::*, vfs::*};
