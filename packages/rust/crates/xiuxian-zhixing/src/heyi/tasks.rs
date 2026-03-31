use super::ZhixingHeyi;
use super::constants::{ATTR_JOURNAL_CARRYOVER, ATTR_TIMER_REMINDED, ATTR_TIMER_SCHEDULED};
use crate::Result;
use crate::journal::JournalEntry;
use serde_json::json;
use xiuxian_wendao::{Entity, EntityType};

const TASK_TITLE_LIMIT: usize = 30;
const TASK_TITLE_PREFIX: usize = 27;

fn manifest_task_title(input: &str) -> String {
    let mut chars = input.chars();
    let prefix: String = chars.by_ref().take(TASK_TITLE_PREFIX).collect();
    if chars.next().is_some() {
        format!("{prefix}...")
    } else {
        input.to_string()
    }
}

fn build_task_entity(
    id: String,
    title: String,
    content: String,
    scheduled_at: Option<String>,
) -> Entity {
    let mut entity = Entity::new(id, title, EntityType::Other("Task".to_string()), content);
    entity
        .metadata
        .insert(ATTR_JOURNAL_CARRYOVER.to_string(), json!(0));
    entity
        .metadata
        .insert(ATTR_TIMER_REMINDED.to_string(), json!(false));
    if let Some(scheduled_at) = scheduled_at {
        entity
            .metadata
            .insert(ATTR_TIMER_SCHEDULED.to_string(), json!(scheduled_at));
    }
    entity
}

impl ZhixingHeyi {
    /// Reflects on a journal entry and manifests it as a task.
    ///
    /// # Errors
    /// Returns an error when journal persistence fails.
    pub async fn reflect(&self, journal: &mut JournalEntry) -> Result<String> {
        self.storage.record_journal(journal).await?;

        let task_name = if journal.content.chars().count() > TASK_TITLE_LIMIT {
            manifest_task_title(&journal.content)
        } else {
            journal.content.clone()
        };

        let task_entity = build_task_entity(
            format!("task:{}", journal.id),
            task_name.clone(),
            journal.content.clone(),
            None,
        );
        if let Err(error) = self.graph.add_entity(task_entity) {
            log::error!("Failed to update graph: {error}");
        }

        journal.processed = true;
        Ok(format!("Vow manifested: '{task_name}'."))
    }

    /// Adds a task with an optional scheduled time.
    ///
    /// # Errors
    /// Returns an error when journal persistence fails.
    pub async fn add_task(&self, title: &str, scheduled_at: Option<String>) -> Result<String> {
        let journal = JournalEntry::new(title.to_string());
        self.storage.record_journal(&journal).await?;

        let scheduled_at_for_queue = scheduled_at.clone();
        let task_name = if title.chars().count() > TASK_TITLE_LIMIT {
            manifest_task_title(title)
        } else {
            title.to_string()
        };

        let task_entity = build_task_entity(
            format!("task:{}", journal.id),
            task_name.clone(),
            title.to_string(),
            scheduled_at,
        );
        if let Err(error) = self.graph.add_entity(task_entity) {
            log::error!("Failed to update graph: {error}");
        }

        if let (Some(reminder_queue), Some(scheduled_at)) = (
            self.reminder_queue.as_ref(),
            scheduled_at_for_queue.as_deref(),
        ) && let Err(error) = reminder_queue.enqueue_task(
            &format!("task:{}", journal.id),
            &task_name,
            Some(title),
            scheduled_at,
            None,
        ) {
            log::warn!("Failed to enqueue scheduled task reminder: {error}");
        }

        Ok(format!("Vow manifested: '{task_name}'."))
    }
}
