use super::ZhixingHeyi;
use super::constants::{ATTR_TIMER_RECIPIENT, ATTR_TIMER_REMINDED, ATTR_TIMER_SCHEDULED};
use chrono::{DateTime, Duration, Utc};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

/// Notification payload emitted by the timer watcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReminderSignal {
    /// Task title rendered in reminder message.
    pub title: String,
    /// Delivery target (for example `telegram:1304799691`).
    pub recipient: Option<String>,
}

impl ZhixingHeyi {
    /// Starts the background timer watcher to proactively monitor scheduled tasks.
    /// This fully encapsulates the domain logic of Agenda/Journal timeouts
    /// and uses an abstract channel to push notifications back to the host system.
    #[must_use]
    pub fn start_timer_watcher(
        self: Arc<Self>,
        notifier: Sender<ReminderSignal>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let reminders = self.poll_reminders();
                for title in reminders {
                    if notifier.send(title).await.is_err() {
                        log::warn!("Timer watcher notification channel closed, stopping watcher.");
                        break;
                    }
                }
            }
        })
    }

    /// Checks for tasks that need immediate reminders in local time.
    ///
    /// Tasks scheduled within the next 15 minutes are returned once and then
    /// marked with `timer:reminded=true`.
    #[must_use]
    pub fn poll_reminders(&self) -> Vec<ReminderSignal> {
        let tasks = self.graph.get_entities_by_type("OTHER(Task)");
        let now_local = Utc::now().with_timezone(&self.time_zone);
        let mut reminders = Vec::new();
        let mut pending_updates = Vec::new();

        for entity in tasks {
            let scheduled = entity
                .metadata
                .get(ATTR_TIMER_SCHEDULED)
                .and_then(serde_json::Value::as_str);
            let reminded = entity
                .metadata
                .get(ATTR_TIMER_REMINDED)
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false);
            let recipient = entity
                .metadata
                .get(ATTR_TIMER_RECIPIENT)
                .and_then(serde_json::Value::as_str)
                .map(ToString::to_string);

            let Some(scheduled) = scheduled else {
                continue;
            };
            let Ok(scheduled_at_utc) = DateTime::parse_from_rfc3339(scheduled) else {
                continue;
            };

            let scheduled_local = scheduled_at_utc.with_timezone(&self.time_zone);
            let reminder_window_start = scheduled_local - Duration::minutes(15);
            if !reminded && now_local >= reminder_window_start && now_local < scheduled_local {
                reminders.push(ReminderSignal {
                    title: entity.name.clone(),
                    recipient,
                });
                let mut updated = entity.clone();
                updated
                    .metadata
                    .insert(ATTR_TIMER_REMINDED.to_string(), json!(true));
                pending_updates.push(updated);
            }
        }

        for updated in pending_updates {
            if let Err(error) = self.graph.add_entity(updated) {
                log::warn!("Failed to update reminder state in graph: {error}");
            }
        }

        reminders
    }
}
