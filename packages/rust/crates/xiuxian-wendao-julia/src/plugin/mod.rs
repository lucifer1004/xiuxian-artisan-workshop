mod discovery;
mod entry;
mod linking;
mod project;
mod sources;
#[cfg(test)]
pub(crate) mod test_support;
mod transport;

pub use entry::JuliaRepoIntelligencePlugin;
pub use entry::register_into;
pub use transport::{
    JULIA_ARROW_RESPONSE_SCHEMA_VERSION, build_julia_flight_transport_client,
    process_julia_flight_batches, process_julia_flight_batches_for_repository,
    validate_julia_arrow_response_batches,
};
