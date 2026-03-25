pub(crate) mod analysis;
pub(crate) mod contracts;
pub(crate) mod docs;
pub(crate) mod graph;
pub(crate) mod inventory;
pub(crate) mod repo;
pub(crate) mod search;
pub(crate) mod ui;
pub(crate) mod vfs;

#[cfg(test)]
mod tests;

pub use contracts::*;
pub use inventory::WENDAO_GATEWAY_ROUTE_CONTRACTS;
