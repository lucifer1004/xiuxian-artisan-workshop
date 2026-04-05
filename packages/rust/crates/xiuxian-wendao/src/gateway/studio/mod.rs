//! Studio API gateway for Qianji frontend.
//!
//! Provides HTTP endpoints for VFS operations, graph queries, and UI configuration.

pub mod types;

#[cfg(feature = "zhenfa-router")]
mod analysis;
#[cfg(feature = "zhenfa-router")]
mod pathing;
/// Performance fixtures and helpers for Studio gateway benchmarks.
#[cfg(all(feature = "zhenfa-router", feature = "performance"))]
pub mod perf_support;
#[cfg(feature = "zhenfa-router")]
pub mod repo_index;
#[cfg(feature = "zhenfa-router")]
pub mod router;
#[cfg(feature = "zhenfa-router")]
pub(crate) mod search;
#[cfg(feature = "zhenfa-router")]
pub mod symbol_index;
#[cfg(feature = "zhenfa-router")]
mod vfs;

#[cfg(feature = "zhenfa-router")]
pub use router::{GatewayState, StudioState, studio_router, studio_routes};
#[cfg(all(feature = "zhenfa-router", test))]
pub(crate) use search::build_ast_index;
#[cfg(feature = "zhenfa-router")]
pub use search::handlers::{
    StudioRepoSearchFlightRouteProvider, bootstrap_sample_repo_search_content,
    build_repo_search_flight_service, build_repo_search_flight_service_with_weights,
    build_studio_flight_service, build_studio_flight_service_for_roots,
    build_studio_flight_service_for_roots_with_weights, build_studio_flight_service_with_weights,
};

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(all(test, feature = "zhenfa-router"))]
#[path = "../../../tests/unit/studio_vfs_performance.rs"]
mod studio_vfs_performance_tests;

#[cfg(all(test, feature = "zhenfa-router"))]
#[path = "../../../tests/unit/studio_repo_sync_api/mod.rs"]
mod studio_repo_sync_api_tests;
