//! External Julia Repo Intelligence plugin for `xiuxian-wendao`.

mod plugin;

pub use plugin::{
    JULIA_ARROW_RESPONSE_SCHEMA_VERSION, JuliaRepoIntelligencePlugin,
    build_julia_arrow_transport_client, process_julia_arrow_batches,
    process_julia_arrow_batches_for_repository, register_into,
    validate_julia_arrow_response_batches,
};
