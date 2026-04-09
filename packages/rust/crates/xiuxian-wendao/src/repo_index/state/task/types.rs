use std::time::Duration;

use crate::analyzers::{RegisteredRepository, RepoIntelligenceError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepoIndexTaskPriority {
    Background,
    Interactive,
}

#[derive(Debug, Clone)]
pub(crate) struct RepoIndexTask {
    pub(crate) repository: RegisteredRepository,
    pub(crate) refresh: bool,
    pub(crate) fingerprint: String,
    pub(crate) priority: RepoIndexTaskPriority,
    pub(crate) retry_count: usize,
}

#[derive(Debug)]
pub(crate) enum RepoTaskOutcome {
    Success {
        revision: Option<String>,
    },
    Failure {
        revision: Option<String>,
        error: RepoIntelligenceError,
    },
    Requeued {
        task: RepoIndexTask,
        error: RepoIntelligenceError,
    },
    Skipped,
}

#[derive(Debug)]
pub(crate) struct RepoTaskFeedback {
    pub(crate) repo_id: String,
    pub(crate) control_elapsed: Duration,
    pub(crate) outcome: RepoTaskOutcome,
}

impl RepoTaskFeedback {
    pub(crate) fn new(repo_id: String, elapsed: Duration, outcome: RepoTaskOutcome) -> Self {
        Self {
            repo_id,
            control_elapsed: elapsed,
            outcome,
        }
    }

    pub(crate) fn with_control_elapsed(
        repo_id: String,
        elapsed: Duration,
        control_elapsed: Duration,
        outcome: RepoTaskOutcome,
    ) -> Self {
        Self {
            repo_id,
            control_elapsed: control_elapsed.min(elapsed),
            outcome,
        }
    }
}

#[cfg(test)]
#[path = "../../../../tests/unit/repo_index/state/task/types.rs"]
mod tests;
