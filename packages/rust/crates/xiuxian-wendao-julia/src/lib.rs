//! External Julia Repo Intelligence plugin for `xiuxian-wendao`.

mod plugin;

pub use plugin::{JuliaRepoIntelligencePlugin, build_julia_arrow_transport_client, register_into};
