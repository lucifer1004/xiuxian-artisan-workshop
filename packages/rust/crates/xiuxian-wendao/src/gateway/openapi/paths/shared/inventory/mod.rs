//! Stable route inventory for the Wendao gateway surface.

mod core;
mod docs;
mod graph;
mod projected;
mod repo;
mod routes;
mod search;
mod ui;
mod vfs;

pub use routes::WENDAO_GATEWAY_ROUTE_CONTRACTS;
#[cfg(test)]
pub(crate) use ui::UI_PLUGIN_ARTIFACT;
