//! Repository analysis endpoint handlers for Studio API.

mod doc_coverage;
pub(crate) mod flight;
mod index_flight;
mod index_status_flight;
mod overview;
mod overview_flight;
mod projected_page_index_tree_flight;
mod refine_doc_flight;
mod search;
mod service;
mod sync;
mod sync_flight;

pub use doc_coverage::doc_coverage;
pub(crate) use flight::StudioRepoDocCoverageFlightRouteProvider;
pub(crate) use index_flight::StudioRepoIndexFlightRouteProvider;
pub(crate) use index_status_flight::StudioRepoIndexStatusFlightRouteProvider;
pub(crate) use index_status_flight::repo_index_status_response_with_diagnostics;
pub use overview::overview;
pub(crate) use overview_flight::StudioRepoOverviewFlightRouteProvider;
pub(crate) use projected_page_index_tree_flight::StudioRepoProjectedPageIndexTreeFlightRouteProvider;
pub(crate) use refine_doc_flight::StudioRefineDocFlightRouteProvider;
pub use search::{example_search, import_search, module_search, symbol_search};
pub use sync::sync;
pub(crate) use sync_flight::StudioRepoSyncFlightRouteProvider;
