//! Reusable repository substrate for materialization, revision resolution, and
//! checkout observation.

mod backend;
mod error;
mod layout;
mod lock;
mod metadata;
mod revision;
mod spec;
mod sync;

pub use error::{RepoError, RepoErrorKind};
pub use layout::{managed_checkout_root_for, managed_mirror_root_for, sanitize_repo_id};
pub use lock::{
    ManagedCheckoutLock, acquire_managed_checkout_lock, acquire_managed_checkout_lock_with_policy,
    checkout_lock_max_wait_with_lookup, is_descriptor_pressure_error, managed_lock_path_for,
};
pub use metadata::{
    LocalCheckoutMetadata, ManagedRemoteProbeState, ManagedRemoteProbeStatus,
    clear_managed_remote_probe_state, discover_checkout_metadata,
    discover_managed_remote_probe_state, record_managed_remote_probe_failure,
    record_managed_remote_probe_state,
};
pub use spec::{RepoRefreshPolicy, RepoSpec, RevisionSelector};
pub use sync::{
    MaterializedRepo, RepoDriftState, RepoLifecycleState, RepoSourceKind, SyncMode,
    resolve_repository_source,
};
