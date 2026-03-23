//! Virtual File System (VFS) orchestration for Studio API.

mod categories;
mod content;
mod filters;
mod navigation;
mod roots;
mod scan;

#[cfg(test)]
mod tests;

pub(crate) use content::{get_entry, read_content};
pub(crate) use navigation::resolve_navigation_target;
pub(crate) use roots::list_root_entries;
pub(crate) use scan::scan_roots;
