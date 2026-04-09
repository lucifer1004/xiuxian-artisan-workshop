#[cfg(test)]
mod tests {
    use chrono::Local;
    use tempfile::tempdir;
    use tokio::fs;
    use uuid::Uuid;
    use xiuxian_zhixing::storage::MarkdownStorage;
    use xiuxian_zhixing::{AgendaEntry, JournalEntry};

    #[tokio::test]
    async fn test_markdown_journal_recording() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempdir()?;
        let storage = MarkdownStorage::new(tmp.path().to_path_buf());

        let mut journal = JournalEntry::new("Reflected on the sword technique today.".to_string());
        let test_id = Uuid::new_v4();
        journal.id = test_id;

        storage.record_journal(&journal).await?;

        let date_str = Local::now().format("%Y-%m-%d").to_string();
        let file_path = tmp.path().join("journal").join(format!("{date_str}.md"));

        assert!(file_path.exists(), "Journal file should be created");
        let content = fs::read_to_string(file_path).await?;
        assert!(content.contains("Reflected on the sword technique"));
        assert!(content.contains(&test_id.to_string()));
        Ok(())
    }

    #[tokio::test]
    async fn test_markdown_task_recording() -> Result<(), Box<dyn std::error::Error>> {
        let tmp = tempdir()?;
        let storage = MarkdownStorage::new(tmp.path().to_path_buf());

        let mut task = AgendaEntry::new("Learn Rust Async".to_string());
        let test_id = Uuid::new_v4();
        task.id = test_id;

        storage.record_task(&task).await?;

        let date_str = Local::now().format("%Y-%m-%d").to_string();
        let file_path = tmp.path().join("agenda").join(format!("{date_str}.md"));

        assert!(file_path.exists(), "Agenda file should be created");
        let content = fs::read_to_string(file_path).await?;
        assert!(content.contains("- [ ] Learn Rust Async"));
        assert!(content.contains(&test_id.to_string()));
        Ok(())
    }
}
