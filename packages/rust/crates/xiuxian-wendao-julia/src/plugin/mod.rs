mod discovery;
mod entry;
mod linking;
mod project;
mod sources;
mod transport;

pub use entry::JuliaRepoIntelligencePlugin;
pub use entry::register_into;
pub use transport::build_julia_arrow_transport_client;
