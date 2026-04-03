//! Git checkout materialization and synchronization.

mod lock;
mod managed;
mod metadata;
mod namespace;
mod refs;
mod source;
mod types;

pub(crate) use metadata::ManagedRemoteProbeStatus;
pub use metadata::discover_checkout_metadata;
pub(crate) use metadata::discover_managed_remote_probe_state;
#[cfg(test)]
pub(crate) use metadata::record_managed_remote_probe_failure;
#[cfg(test)]
pub(crate) use metadata::record_managed_remote_probe_state;
pub use source::resolve_repository_source;
pub use types::{
    CheckoutSyncMode, LocalCheckoutMetadata, RepositoryLifecycleState, RepositorySyncMode,
    ResolvedRepositorySource, ResolvedRepositorySourceKind,
};

#[cfg(test)]
mod tests;
