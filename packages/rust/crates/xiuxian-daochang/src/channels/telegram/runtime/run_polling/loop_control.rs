use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;

use crate::agent::Agent;
use crate::channels::telegram::runtime::dispatch::ForegroundInterruptController;
use crate::channels::telegram::runtime::telemetry::{
    emit_runtime_snapshot, snapshot_interval_from_env,
};
use crate::channels::telegram::runtime_config::TelegramRuntimeConfig;
use crate::channels::traits::{Channel, ChannelMessage};
use crate::jobs::{JobCompletion, JobManager};

use super::super::jobs::{handle_inbound_message_with_interrupt, push_background_completion};

pub(super) async fn run_polling_event_loop(
    inbound_rx: &mut mpsc::Receiver<ChannelMessage>,
    completion_rx: &mut mpsc::Receiver<JobCompletion>,
    inbound_tx: &mpsc::Sender<ChannelMessage>,
    channel_for_send: &Arc<dyn Channel>,
    foreground_tx: &mpsc::Sender<ChannelMessage>,
    interrupt_controller: &ForegroundInterruptController,
    job_manager: &Arc<JobManager>,
    agent: &Arc<Agent>,
    runtime_config: TelegramRuntimeConfig,
) {
    let mut snapshot_tick = snapshot_interval_from_env().map(|period| {
        let mut interval = tokio::time::interval(period);
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        interval
    });
    if let Some(interval) = snapshot_tick.as_mut() {
        let _ = interval.tick().await;
    }

    loop {
        tokio::select! {
            maybe_msg = inbound_rx.recv() => {
                let Some(msg) = maybe_msg else {
                    break;
                };
                if !handle_inbound_message_with_interrupt(
                    msg,
                    channel_for_send,
                    foreground_tx,
                    interrupt_controller,
                    job_manager,
                    agent,
                    runtime_config.foreground_queue_mode,
                )
                .await {
                    break;
                }
            }
            maybe_completion = completion_rx.recv() => {
                let Some(completion) = maybe_completion else {
                    continue;
                };
                push_background_completion(channel_for_send, completion).await;
            }
            _ = tokio::signal::ctrl_c() => {
                println!("Shutting down...");
                break;
            }
            _ = async {
                if let Some(interval) = snapshot_tick.as_mut() {
                    let _ = interval.tick().await;
                }
            }, if snapshot_tick.is_some() => {
                emit_runtime_snapshot("polling", inbound_tx, foreground_tx, runtime_config);
            }
        }
    }
}
