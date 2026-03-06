use std::sync::Arc;

use crate::agent::Agent;
use crate::channels::traits::Channel;
use crate::jobs::{JobCompletion, JobCompletionKind, append_completion_to_parent_session};

use super::events::{
    EVENT_DISCORD_COMMAND_BACKGROUND_COMPLETION_FAILED_REPLIED,
    EVENT_DISCORD_COMMAND_BACKGROUND_COMPLETION_SUCCEEDED_REPLIED,
    EVENT_DISCORD_COMMAND_BACKGROUND_COMPLETION_TIMED_OUT_REPLIED,
};
use super::send::send_completion;

pub(in super::super::super) async fn push_background_completion(
    channel: &Arc<dyn Channel>,
    agent: &Arc<Agent>,
    completion: JobCompletion,
) {
    let (event_name, message) = match &completion.kind {
        JobCompletionKind::Succeeded { output } => (
            EVENT_DISCORD_COMMAND_BACKGROUND_COMPLETION_SUCCEEDED_REPLIED,
            format!(
                "Background job `{}` completed.\n\n{}",
                completion.job_id, output
            ),
        ),
        JobCompletionKind::Failed { error } => (
            EVENT_DISCORD_COMMAND_BACKGROUND_COMPLETION_FAILED_REPLIED,
            format!("Background job `{}` failed: {}", completion.job_id, error),
        ),
        JobCompletionKind::TimedOut { timeout_secs } => (
            EVENT_DISCORD_COMMAND_BACKGROUND_COMPLETION_TIMED_OUT_REPLIED,
            format!(
                "Background job `{}` timed out after {}s.",
                completion.job_id, timeout_secs
            ),
        ),
    };
    append_completion_to_parent_session(agent.as_ref(), &completion, &message).await;
    send_completion(
        channel,
        &completion.recipient,
        message,
        event_name,
        completion.parent_session_key(),
    )
    .await;
}
