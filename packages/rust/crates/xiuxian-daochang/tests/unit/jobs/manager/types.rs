use super::{JobCompletion, JobCompletionKind, truncate_for_status};

#[test]
fn parent_session_key_returns_parent_session_id() {
    let completion = JobCompletion {
        job_id: "job-1".to_string(),
        recipient: "telegram:-1".to_string(),
        parent_session_id: "telegram:-1:42".to_string(),
        kind: JobCompletionKind::TimedOut { timeout_secs: 30 },
    };

    assert_eq!(completion.parent_session_key(), "telegram:-1:42");
}

#[test]
fn truncate_for_status_adds_ellipsis_when_needed() {
    assert_eq!(truncate_for_status("abcdefghij", 6), "abc...");
    assert_eq!(truncate_for_status("short", 6), "short");
}
