pub(crate) mod contracts;
pub(crate) mod inventory;

#[cfg(test)]
#[path = "../../../../../tests/unit/gateway/openapi/paths/shared/mod.rs"]
mod tests;

pub use contracts::*;
pub use inventory::WENDAO_GATEWAY_ROUTE_CONTRACTS;
