use super::{
    ManagedRemoteProbeStatus, clear_managed_remote_probe_state,
    discover_managed_remote_probe_state, record_managed_remote_probe_failure,
    record_managed_remote_probe_state,
};

fn tempdir_or_panic() -> tempfile::TempDir {
    tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"))
}

fn discover_state_or_panic(mirror_root: &std::path::Path) -> super::ManagedRemoteProbeState {
    let Some(state) = discover_managed_remote_probe_state(mirror_root) else {
        panic!("expected managed remote probe state");
    };
    state
}

#[test]
fn managed_remote_probe_state_round_trips() {
    let temp = tempdir_or_panic();
    record_managed_remote_probe_state(temp.path(), Some("rev-1"))
        .unwrap_or_else(|error| panic!("record probe state: {error}"));

    let state = discover_state_or_panic(temp.path());
    assert_eq!(state.status, ManagedRemoteProbeStatus::Success);
    assert_eq!(state.target_revision.as_deref(), Some("rev-1"));
    assert_eq!(state.last_success_target_revision.as_deref(), Some("rev-1"));
}

#[test]
fn managed_remote_probe_failure_preserves_last_success() {
    let temp = tempdir_or_panic();
    record_managed_remote_probe_state(temp.path(), Some("rev-1"))
        .unwrap_or_else(|error| panic!("record probe state: {error}"));
    record_managed_remote_probe_failure(temp.path(), "operation timed out", true)
        .unwrap_or_else(|error| panic!("record probe failure: {error}"));

    let state = discover_state_or_panic(temp.path());
    assert_eq!(state.status, ManagedRemoteProbeStatus::RetryableFailure);
    assert_eq!(state.target_revision, None);
    assert_eq!(state.last_success_target_revision.as_deref(), Some("rev-1"));
}

#[test]
fn managed_remote_probe_state_clears() {
    let temp = tempdir_or_panic();
    record_managed_remote_probe_state(temp.path(), Some("rev-1"))
        .unwrap_or_else(|error| panic!("record probe state: {error}"));
    clear_managed_remote_probe_state(temp.path())
        .unwrap_or_else(|error| panic!("clear probe state: {error}"));

    assert_eq!(discover_managed_remote_probe_state(temp.path()), None);
}
