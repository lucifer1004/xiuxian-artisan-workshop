use std::time::Duration;

use super::{RepoTaskFeedback, RepoTaskOutcome};

#[test]
fn repo_task_feedback_defaults_control_elapsed_to_total_elapsed() {
    let feedback = RepoTaskFeedback::new(
        "alpha/repo".to_string(),
        Duration::from_millis(150),
        RepoTaskOutcome::Skipped,
    );

    assert_eq!(feedback.control_elapsed, Duration::from_millis(150));
}

#[test]
fn repo_task_feedback_clamps_control_elapsed_to_total_elapsed() {
    let feedback = RepoTaskFeedback::with_control_elapsed(
        "alpha/repo".to_string(),
        Duration::from_millis(150),
        Duration::from_millis(300),
        RepoTaskOutcome::Skipped,
    );

    assert_eq!(feedback.control_elapsed, Duration::from_millis(150));
}
