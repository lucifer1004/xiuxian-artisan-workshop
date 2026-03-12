use std::sync::Arc;

use crate::agent::Agent;
use crate::channels::managed_commands::{
    detect_managed_control_command, detect_managed_slash_command,
};
use crate::channels::managed_runtime::ForegroundQueueMode;
use crate::channels::managed_runtime::parsing::is_stop_command;
use crate::channels::managed_runtime::turn::{build_session_id, compose_turn_content};
use crate::channels::traits::{Channel, ChannelMessage};
use crate::jobs::JobManager;

use super::super::ForegroundInterruptController;
use super::generation::begin_active_generation;
use super::stop::try_handle_stop_command;
use super::support::{log_inbound_user_message, log_preempted_turn};
use super::turn::{
    ForegroundTurnInput, render_foreground_turn_reply, run_foreground_turn_with_typing,
};
use crate::channels::discord::runtime::managed::handle_inbound_managed_command;

pub(in crate::channels::discord::runtime) async fn process_discord_message_with_interrupt(
    agent: Arc<Agent>,
    channel: Arc<dyn Channel>,
    msg: ChannelMessage,
    job_manager: &Arc<JobManager>,
    turn_timeout_secs: u64,
    foreground_queue_mode: ForegroundQueueMode,
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

    if foreground_queue_mode.should_interrupt_on_new_message() {
        log_preempted_turn(interrupt_controller, &session_id, &msg);
    }
    log_inbound_user_message(&msg);
    let (interrupt_rx, _active_generation_guard, interrupt_generation) =
        begin_active_generation(interrupt_controller, &session_id);
    let turn_content = compose_turn_content(&msg);
    let turn_input = ForegroundTurnInput {
        recipient: &msg.recipient,
        session_id: &session_id,
        content: turn_content.as_str(),
        turn_timeout_secs,
        interrupt_rx,
        interrupt_generation,
    };
    let result = run_foreground_turn_with_typing(channel.as_ref(), agent.clone(), turn_input).await;
    if let Some(reply) = render_foreground_turn_reply(result, &msg, turn_timeout_secs) {
        super::turn::send_discord_reply(channel.as_ref(), &msg, &reply).await;
    }
}
