//! Integration tests for Zhixing-Heyi orchestration flows.

use chrono::{Duration, Utc};
use serde_json::json;
use std::error::Error;
use std::fs;
use std::sync::Arc;
use tempfile::tempdir;
use xiuxian_qianhuan::ManifestationInterface;
use xiuxian_wendao::entity::{Entity, EntityType};
use xiuxian_wendao::graph::KnowledgeGraph;
use xiuxian_zhixing::ATTR_TIMER_REMINDED;
use xiuxian_zhixing::ATTR_TIMER_SCHEDULED;
use xiuxian_zhixing::ReminderSignal;
use xiuxian_zhixing::ZhixingHeyi;
use xiuxian_zhixing::storage::MarkdownStorage;

struct EchoManifestation;

impl ManifestationInterface for EchoManifestation {
    fn render_template(
        &self,
        _template_name: &str,
        data: serde_json::Value,
    ) -> anyhow::Result<String> {
        Ok(data.to_string())
    }

    fn inject_context(&self, state_context: &str) -> String {
        state_context.to_string()
    }
}

#[test]
fn test_time_zone_parsing() -> std::result::Result<(), Box<dyn Error>> {
    let graph = Arc::new(KnowledgeGraph::new());
    let tmp = tempdir()?;
    let storage = Arc::new(MarkdownStorage::new(tmp.path().to_path_buf()));
    let manifestation = Arc::new(EchoManifestation);

    let heyi = ZhixingHeyi::new(graph, manifestation, storage, "test".to_string(), "UTC")?;
    assert_eq!(heyi.time_zone.to_string(), "UTC");
    Ok(())
}

#[test]
fn test_invalid_time_zone_returns_config_error() -> std::result::Result<(), Box<dyn Error>> {
    let graph = Arc::new(KnowledgeGraph::new());
    let tmp = tempdir()?;
    let storage = Arc::new(MarkdownStorage::new(tmp.path().to_path_buf()));
    let manifestation = Arc::new(EchoManifestation);

    let result = ZhixingHeyi::new(
        graph,
        manifestation,
        storage,
        "test".to_string(),
        "Invalid/Zone",
    );
    match result {
        Ok(_) => panic!("Expected invalid time-zone constructor to fail"),
        Err(error) => assert!(error.to_string().contains("Invalid time zone")),
    }
    Ok(())
}

#[test]
fn test_reminder_trigger_logic() -> std::result::Result<(), Box<dyn Error>> {
    let graph = Arc::new(KnowledgeGraph::new());
    let tmp = tempdir()?;
    let storage = Arc::new(MarkdownStorage::new(tmp.path().to_path_buf()));
    let manifestation = Arc::new(EchoManifestation);
    let heyi = ZhixingHeyi::new(
        Arc::clone(&graph),
        manifestation,
        storage,
        "test".to_string(),
        "UTC",
    )?;

    let scheduled_time = Utc::now() + Duration::minutes(10);
    let mut entity = Entity::new(
        "task:reminder-trigger-logic".to_string(),
        "Trigger Task".to_string(),
        EntityType::Other("Task".to_string()),
        String::new(),
    );
    entity.metadata.insert(
        ATTR_TIMER_SCHEDULED.to_string(),
        json!(scheduled_time.to_rfc3339()),
    );
    entity
        .metadata
        .insert(ATTR_TIMER_REMINDED.to_string(), json!(false));
    graph.add_entity(entity)?;

    let reminders = heyi.poll_reminders();
    assert_eq!(reminders.len(), 1);
    assert_eq!(reminders[0].title, "Trigger Task");
    assert_eq!(reminders[0].recipient, None);

    let reminders_second = heyi.poll_reminders();
    assert_eq!(reminders_second.len(), 0);
    Ok(())
}

#[test]
fn test_render_agenda_uses_human_local_time() -> std::result::Result<(), Box<dyn Error>> {
    let graph = Arc::new(KnowledgeGraph::new());
    let tmp = tempdir()?;
    let storage = Arc::new(MarkdownStorage::new(tmp.path().to_path_buf()));
    let manifestation = Arc::new(EchoManifestation);
    let heyi = ZhixingHeyi::new(
        Arc::clone(&graph),
        manifestation,
        storage,
        "test".to_string(),
        "America/Los_Angeles",
    )?;

    let mut entity = Entity::new(
        "task:render-human-time".to_string(),
        "Render Human Time Task".to_string(),
        EntityType::Other("Task".to_string()),
        String::new(),
    );
    entity.metadata.insert(
        ATTR_TIMER_SCHEDULED.to_string(),
        json!("2026-02-26T06:09:00+00:00"),
    );
    entity
        .metadata
        .insert(ATTR_TIMER_REMINDED.to_string(), json!(false));
    graph.add_entity(entity)?;

    let rendered = heyi.render_agenda()?;
    assert!(
        rendered.contains("2026-02-25 10:09 PM"),
        "agenda output should render local human time: {rendered}"
    );
    assert!(
        !rendered.contains("2026-02-26T06:09:00+00:00"),
        "agenda output should not leak raw RFC3339 metadata: {rendered}"
    );
    Ok(())
}

#[test]
fn test_render_agenda_prefers_today_journal_note_from_wendao()
-> std::result::Result<(), Box<dyn Error>> {
    let graph = Arc::new(KnowledgeGraph::new());
    let tmp = tempdir()?;
    let storage = Arc::new(MarkdownStorage::new(tmp.path().to_path_buf()));
    let manifestation = Arc::new(EchoManifestation);
    let heyi = ZhixingHeyi::new(
        Arc::clone(&graph),
        manifestation,
        storage,
        "test".to_string(),
        "America/Los_Angeles",
    )?;

    let local_date = Utc::now()
        .with_timezone(&heyi.time_zone)
        .format("%Y-%m-%d")
        .to_string();
    let journal_dir = tmp.path().join("journal");
    fs::create_dir_all(&journal_dir)?;
    let note_rel_path = format!("journal/{local_date}.md");
    let note_path = tmp.path().join(&note_rel_path);
    fs::write(
        &note_path,
        "## [21:11:15] Reflection\n检查timer通知\n<!-- id: test, tags: [] -->\n",
    )?;

    let rendered = heyi.render_agenda()?;
    assert!(
        rendered.contains(&format!("# Daily Agenda ({local_date})")),
        "agenda output should include local-date agenda heading: {rendered}"
    );
    assert!(
        !rendered.contains("Semantic query:"),
        "agenda output should not leak internal search diagnostics: {rendered}"
    );
    assert!(
        rendered.contains("检查timer通知"),
        "agenda output should come from Wendao hit note content: {rendered}"
    );
    assert!(
        !rendered.contains("<!-- id: test, tags: [] -->"),
        "agenda output should not expose html metadata comments: {rendered}"
    );
    assert!(
        !rendered.contains(&note_rel_path),
        "agenda output should not expose note source path: {rendered}"
    );
    Ok(())
}

#[test]
fn test_sync_from_disk_indexes_notebook_into_wendao_graph()
-> std::result::Result<(), Box<dyn Error>> {
    let graph = Arc::new(KnowledgeGraph::new());
    let tmp = tempdir()?;
    let storage = Arc::new(MarkdownStorage::new(tmp.path().to_path_buf()));
    let manifestation = Arc::new(EchoManifestation);
    let heyi = ZhixingHeyi::new(
        Arc::clone(&graph),
        manifestation,
        storage,
        "test".to_string(),
        "UTC",
    )?;

    let journal_dir = tmp.path().join("journal");
    let agenda_dir = tmp.path().join("agenda");
    fs::create_dir_all(&journal_dir)?;
    fs::create_dir_all(&agenda_dir)?;
    fs::write(
        journal_dir.join("2026-02-26.md"),
        "## Reflection\nObserved execution discipline improvement.\n",
    )?;
    fs::write(
        agenda_dir.join("2026-02-26.md"),
        "- [ ] Verify sync path <!-- id: sync-1, journal:carryover: 1 -->\n",
    )?;

    let summary = heyi.sync_from_disk()?;
    assert_eq!(summary.journal_documents, 1);
    assert_eq!(summary.agenda_documents, 1);
    assert_eq!(summary.task_entities, 1);

    let documents = graph.get_entities_by_type("DOCUMENT");
    assert!(
        documents.len() >= 2,
        "sync should include at least agenda/journal documents; got {}",
        documents.len()
    );
    assert!(
        documents
            .iter()
            .any(|entity| entity.name == "Journal 2026-02-26"),
        "journal notebook document should exist after sync"
    );
    assert!(
        documents
            .iter()
            .any(|entity| entity.name == "Agenda 2026-02-26"),
        "agenda notebook document should exist after sync"
    );
    let tasks = graph.get_entities_by_type("OTHER(Task)");
    assert_eq!(tasks.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_add_task_preserves_scheduled_input_on_heyi_surface()
-> std::result::Result<(), Box<dyn Error>> {
    let graph = Arc::new(KnowledgeGraph::new());
    let tmp = tempdir()?;
    let storage = Arc::new(MarkdownStorage::new(tmp.path().to_path_buf()));
    let manifestation = Arc::new(EchoManifestation);
    let heyi = ZhixingHeyi::new(
        Arc::clone(&graph),
        manifestation,
        storage,
        "test".to_string(),
        "America/Los_Angeles",
    )?;

    let response = heyi
        .add_task(
            "Normalize local time",
            Some("2026-02-25 10:09 PM".to_string()),
        )
        .await?;

    let tasks = graph.get_entities_by_type("OTHER(Task)");
    let has_expected_schedule = tasks.iter().any(|task| {
        task.metadata
            .get(ATTR_TIMER_SCHEDULED)
            .and_then(serde_json::Value::as_str)
            == Some("2026-02-25 10:09 PM")
    });
    assert!(has_expected_schedule);
    assert!(response.contains("Normalize local time"));
    Ok(())
}

#[tokio::test]
async fn test_add_task_accepts_unparsed_scheduled_input_on_heyi_surface()
-> std::result::Result<(), Box<dyn Error>> {
    let graph = Arc::new(KnowledgeGraph::new());
    let tmp = tempdir()?;
    let storage = Arc::new(MarkdownStorage::new(tmp.path().to_path_buf()));
    let manifestation = Arc::new(EchoManifestation);
    let heyi = ZhixingHeyi::new(
        Arc::clone(&graph),
        manifestation,
        storage,
        "test".to_string(),
        "America/Los_Angeles",
    )?;

    let marker = "Reject invalid time marker";
    let response = heyi
        .add_task(marker, Some("blorp-not-a-time".to_string()))
        .await?;
    assert!(response.contains(marker));

    let tasks = graph.get_entities_by_type("OTHER(Task)");
    assert_eq!(tasks.len(), 1);
    assert_eq!(
        tasks[0]
            .metadata
            .get(ATTR_TIMER_SCHEDULED)
            .and_then(serde_json::Value::as_str),
        Some("blorp-not-a-time")
    );

    Ok(())
}

#[test]
fn test_render_reminder_notice_markdown_uses_live_signal_fields()
-> std::result::Result<(), Box<dyn Error>> {
    let tmp = tempdir()?;
    let storage = Arc::new(MarkdownStorage::new(tmp.path().to_path_buf()));
    let manifestation = Arc::new(EchoManifestation);
    let heyi = ZhixingHeyi::new(
        Arc::new(KnowledgeGraph::new()),
        manifestation,
        storage,
        "test".to_string(),
        "America/Los_Angeles",
    )?;

    let rendered = heyi.render_reminder_notice_markdown(&ReminderSignal {
        task_id: "task:render-from-id".to_string(),
        title: "验证知行提醒模板".to_string(),
        task_brief: Some("检查角色注入文案是否出现并且可读".to_string()),
        scheduled_at: Some("2026-02-26T08:50:00+00:00".to_string()),
        recipient: Some("llm:test".to_string()),
    })?;
    let payload: serde_json::Value = serde_json::from_str(&rendered)?;
    assert_eq!(payload["task_title_mdv2"], json!("验证知行提醒模板"));
    assert_eq!(
        payload["task_brief_mdv2"],
        json!("检查角色注入文案是否出现并且可读")
    );
    assert_eq!(payload["task_id_mdv2"], json!("task:render-from-id"));
    assert_eq!(
        payload["qianhuan"]["state_context"],
        json!("SUCCESS_STREAK")
    );
    assert!(
        payload["scheduled_local_mdv2"]
            .as_str()
            .is_some_and(|value| value.contains("2026-02-26 12:50 AM")),
        "expected local time in rendered payload: {payload}"
    );
    Ok(())
}
