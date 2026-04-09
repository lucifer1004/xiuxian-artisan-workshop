mod core;
/// Shared `FlightSQL` adapter surface over the request-scoped query system.
pub mod flightsql;
/// Shared `GraphQL` adapter surface over the request-scoped query system.
pub mod graphql;
/// Shared `REST` adapter surface over the request-scoped query system.
pub mod rest;
/// Shared `SQL` adapter surface over the request-scoped query system.
pub mod sql;

pub use self::core::SearchQueryService;

#[cfg(test)]
#[path = "../../../tests/unit/search/queries/mod.rs"]
mod tests;
