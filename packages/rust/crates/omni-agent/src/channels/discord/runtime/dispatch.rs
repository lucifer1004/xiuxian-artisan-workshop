use crate::agent::Agent;
use crate::channels::managed_commands::{
    detect_managed_control_command, detect_managed_slash_command,
};
use crate::channels::managed_runtime::parsing::is_stop_command;
use crate::channels::managed_runtime::turn::{
    ForegroundTurnOutcome, build_session_id, run_foreground_turn_with_interrupt,
};
use crate::channels::traits::{Channel, ChannelMessage};
use crate::jobs::JobManager;
use std::sync::Arc;

use super::ForegroundInterruptController;
use super::managed::handle_inbound_managed_command;

const LOG_PREVIEW_LEN: usize = 80;

#[cfg(test)]
pub(super) async fn process_discord_message(
    agent: Arc<Agent>,
    channel: Arc<dyn Channel>,
    msg: ChannelMessage,
    job_manager: &Arc<JobManager>,
    turn_timeout_secs: u64,
) {
    let interrupt_controller = ForegroundInterruptController::default();
    process_discord_message_with_interrupt(
        agent,
        channel,
        msg,
        job_manager,
        turn_timeout_secs,
        &interrupt_controller,
    )
    .await;
}

#[allow(clippy::too_many_lines)]
pub(in crate::channels::discord::runtime) async fn process_discord_message_with_interrupt(
    agent: Arc<Agent>,
    channel: Arc<dyn Channel>,
    msg: ChannelMessage,
    job_manager: &Arc<JobManager>,
    turn_timeout_secs: u64,
    interrupt_controller: &ForegroundInterruptController,
) {
    let session_id = build_session_id(&msg.channel, &msg.session_key);

    if is_stop_command(&msg.content)
        && try_handle_stop_command(
            agent.as_ref(),
            channel.as_ref(),
            &msg,
            &session_id,
            interrupt_controller,
        )
        .await
    {
        return;
    }

    if let Some(control_command) = detect_managed_control_command(&msg.content) {
        tracing::debug!(
            command = control_command.canonical_command(),
            "discord managed control command detected"
        );
    }
    if let Some(slash_command) = detect_managed_slash_command(&msg.content) {
        tracing::debug!(
            command = slash_command.canonical_command(),
            scope = slash_command.scope(),
            "discord managed slash command detected"
        );
    }

    if handle_inbound_managed_command(&agent, &channel, &msg, job_manager).await {
        return;
    }

    if interrupt_controller.interrupt(&session_id) {
        tracing::info!(
            event = "discord.foreground.turn.preempted",
            session_key = %msg.session_key,
            channel = %msg.channel,
            recipient = %msg.recipient,
            sender = %msg.sender,
            "discord active foreground turn interrupted by newer inbound message"
        );
    }

    tracing::info!(
        r#"discord ← User: "{preview}""#,
        preview = log_preview(&msg.content)
    );

    let interrupt_rx = interrupt_controller.begin_generation(&session_id);
    let _active_generation_guard =
        ActiveGenerationGuard::new(interrupt_controller.clone(), session_id.clone());
    let interrupt_generation = *interrupt_rx.borrow();

    if let Err(error) = channel.start_typing(&msg.recipient).await {
        tracing::debug!("discord: failed to start typing: {error}");
    }

    let result = run_foreground_turn_with_interrupt(
        agent.clone(),
        &session_id,
        &msg.content,
        turn_timeout_secs,
        format!("Request timed out after {turn_timeout_secs}s."),
        interrupt_rx,
        interrupt_generation,
        "Request interrupted by a newer instruction.".to_string(),
    )
    .await;

    if let Err(error) = channel.stop_typing(&msg.recipient).await {
        tracing::debug!("discord: failed to stop typing: {error}");
    }

    let reply = match result {
        ForegroundTurnOutcome::Succeeded(output) => output,
        ForegroundTurnOutcome::Failed {
            reply,
            error_chain,
            error_kind,
        } => {
            tracing::error!(
                event = "discord.foreground.turn.failed",
                session_key = %msg.session_key,
                channel = %msg.channel,
                recipient = %msg.recipient,
                sender = %msg.sender,
                error_kind,
                error = %error_chain,
                "discord foreground turn failed"
            );
            reply
        }
        ForegroundTurnOutcome::TimedOut { reply } => {
            tracing::warn!(
                event = "discord.foreground.turn.timeout",
                session_key = %msg.session_key,
                channel = %msg.channel,
                recipient = %msg.recipient,
                sender = %msg.sender,
                timeout_secs = turn_timeout_secs,
                "discord foreground turn timed out"
            );
            reply
        }
        ForegroundTurnOutcome::Interrupted { reply } => {
            tracing::warn!(
                event = "discord.foreground.turn.interrupted",
                session_key = %msg.session_key,
                channel = %msg.channel,
                recipient = %msg.recipient,
                sender = %msg.sender,
                "discord foreground turn interrupted"
            );
            reply
        }
    };

    match channel.send(&reply, &msg.recipient).await {
        Ok(()) => tracing::info!(
            r#"discord → Bot: "{preview}""#,
            preview = log_preview(&reply)
        ),
        Err(error) => tracing::warn!("discord: failed to send reply: {error}"),
    }
}

#[derive(Clone)]
struct ActiveGenerationGuard {
    controller: ForegroundInterruptController,
    session_id: String,
}

impl ActiveGenerationGuard {
    fn new(controller: ForegroundInterruptController, session_id: String) -> Self {
        Self {
            controller,
            session_id,
        }
    }
}

impl Drop for ActiveGenerationGuard {
    fn drop(&mut self) {
        self.controller.end_generation(&self.session_id);
    }
}

async fn try_handle_stop_command(
    agent: &Agent,
    channel: &dyn Channel,
    msg: &ChannelMessage,
    session_id: &str,
    interrupt_controller: &ForegroundInterruptController,
) -> bool {
    let interrupted = interrupt_controller.interrupt(session_id);
    if interrupted
        && let Err(error) = agent
            .append_turn_for_session(
                session_id,
                "[control] /stop",
                "[system] Current foreground generation interrupted by user request.",
            )
            .await
    {
        tracing::warn!(
            session_id = %session_id,
            error = %error,
            "failed to persist discord stop-interrupted marker for session"
        );
    }

    let response = if interrupted {
        "Stop signal sent. Current foreground generation is being interrupted."
    } else {
        "No active foreground generation to stop in this session."
    };

    let event_name = if interrupted {
        "discord.command.session_stop.replied"
    } else {
        "discord.command.session_stop_idle.replied"
    };
    match channel.send(response, &msg.recipient).await {
        Ok(()) => {
            tracing::info!(
                event = event_name,
                session_key = %msg.session_key,
                channel = %msg.channel,
                recipient = %msg.recipient,
                sender = %msg.sender,
                "discord session stop command replied"
            );
        }
        Err(error) => {
            tracing::warn!(
                event = "discord.command.session_stop.reply_failed",
                session_key = %msg.session_key,
                channel = %msg.channel,
                recipient = %msg.recipient,
                sender = %msg.sender,
                error = %error,
                "discord failed to send stop reply"
            );
        }
    }
    true
}

fn log_preview(s: &str) -> String {
    let one_line: String = s.chars().map(|c| if c == '\n' { ' ' } else { c }).collect();
    if one_line.chars().count() > LOG_PREVIEW_LEN {
        format!(
            "{}...",
            one_line
                .chars()
                .take(LOG_PREVIEW_LEN)
                .collect::<String>()
                .trim_end()
        )
    } else {
        one_line
    }
}
