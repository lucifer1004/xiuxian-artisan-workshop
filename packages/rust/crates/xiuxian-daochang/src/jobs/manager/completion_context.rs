use crate::agent::Agent;

use super::types::JobCompletion;

pub(crate) async fn append_completion_to_parent_session(
    agent: &Agent,
    completion: &JobCompletion,
    assistant_message: &str,
) {
    let user_message = format!("[background] job `{}` completion", completion.job_id);
    if let Err(error) = agent
        .append_turn_for_session(
            completion.parent_session_id.as_str(),
            &user_message,
            assistant_message,
        )
        .await
    {
        tracing::warn!(
            session_id = %completion.parent_session_id,
            job_id = %completion.job_id,
            error = %error,
            "failed to persist background completion into parent session"
        );
    }
}
