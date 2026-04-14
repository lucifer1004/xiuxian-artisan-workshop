#[cfg(feature = "search-runtime")]
mod core;
/// Shared `FlightSQL` adapter surface over the request-scoped query system.
#[cfg(feature = "search-runtime")]
pub mod flightsql;
/// Shared `GraphQL` adapter surface over the request-scoped query system.
#[cfg(feature = "search-runtime")]
pub mod graphql;
/// Shared `REST` adapter surface over the request-scoped query system.
#[cfg(feature = "search-runtime")]
pub mod rest;
/// Shared `SQL` adapter surface over the request-scoped query system.
#[cfg(feature = "search-runtime")]
pub mod sql;

#[cfg(feature = "search-runtime")]
pub use self::core::SearchQueryService;

#[cfg(all(test, feature = "search-runtime"))]
#[path = "../../../tests/unit/search/queries/mod.rs"]
mod tests;
