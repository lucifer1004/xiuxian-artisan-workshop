#![allow(clippy::expect_used)]

use crate::types::KnowledgeCategory;
use tempfile::TempDir;

use super::KnowledgeStorage;

fn configure_test_valkey() -> bool {
    if let Ok(url) = std::env::var("VALKEY_URL")
        && !url.trim().is_empty()
    {
        return true;
    }
    false
}

#[tokio::test]
async fn test_storage_creation() {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let storage = KnowledgeStorage::new(temp_dir.path().to_string_lossy().as_ref(), "knowledge");

    assert_eq!(storage.table_name(), "knowledge");
}

#[tokio::test]
async fn test_upsert_count_delete_clear_roundtrip() {
    if !configure_test_valkey() {
        return;
    }
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let storage = KnowledgeStorage::new(temp_dir.path().to_string_lossy().as_ref(), "knowledge");

    storage.init().await.expect("init should succeed");
    storage.clear().await.expect("clear should succeed");

    let entry = crate::types::KnowledgeEntry::new(
        "id-1".to_string(),
        "Rust Pattern".to_string(),
        "Use Result for error handling".to_string(),
        KnowledgeCategory::Pattern,
    )
    .with_tags(vec!["rust".to_string(), "error".to_string()]);
    storage.upsert(&entry).await.expect("upsert should succeed");
    assert_eq!(storage.count().await.expect("count should succeed"), 1);

    let updated = crate::types::KnowledgeEntry::new(
        "id-1".to_string(),
        "Rust Pattern Updated".to_string(),
        "Use anyhow for context-rich errors".to_string(),
        KnowledgeCategory::Pattern,
    )
    .with_tags(vec!["rust".to_string(), "anyhow".to_string()]);
    storage
        .upsert(&updated)
        .await
        .expect("upsert should succeed");
    assert_eq!(storage.count().await.expect("count should succeed"), 1);

    storage.delete("id-1").await.expect("delete should succeed");
    assert_eq!(storage.count().await.expect("count should succeed"), 0);

    storage.upsert(&entry).await.expect("upsert should succeed");
    storage.clear().await.expect("clear should succeed");
    assert_eq!(storage.count().await.expect("count should succeed"), 0);
}

#[tokio::test]
async fn test_text_search_and_stats() {
    if !configure_test_valkey() {
        return;
    }
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let storage = KnowledgeStorage::new(temp_dir.path().to_string_lossy().as_ref(), "knowledge");
    storage.init().await.expect("init should succeed");
    storage.clear().await.expect("clear should succeed");

    let e1 = crate::types::KnowledgeEntry::new(
        "id-a".to_string(),
        "TypeScript Error Handling".to_string(),
        "Typed errors improve maintainability".to_string(),
        KnowledgeCategory::Pattern,
    )
    .with_tags(vec!["typescript".to_string(), "error".to_string()]);
    let e2 = crate::types::KnowledgeEntry::new(
        "id-b".to_string(),
        "Workflow notes".to_string(),
        "This note describes deployment workflow".to_string(),
        KnowledgeCategory::Workflow,
    )
    .with_tags(vec!["deploy".to_string()]);

    storage.upsert(&e1).await.expect("upsert should succeed");
    storage.upsert(&e2).await.expect("upsert should succeed");

    let text_results = storage
        .search_text("typed error", 10)
        .await
        .expect("search_text should succeed");
    assert_eq!(text_results.len(), 1);
    assert_eq!(text_results[0].id, "id-a");

    let vector_results = storage
        .search(&[0.1, 0.3, 0.2, 0.4], 2)
        .await
        .expect("search should succeed");
    assert_eq!(vector_results.len(), 2);

    let stats = storage.stats().await.expect("stats should succeed");
    assert_eq!(stats.total_entries, 2);
    assert_eq!(stats.total_tags, 3, "stats={stats:?}");
    assert_eq!(stats.entries_by_category.get("patterns"), Some(&1));
    assert_eq!(stats.entries_by_category.get("workflows"), Some(&1));
    assert!(stats.last_updated.is_some());
}

#[tokio::test]
async fn test_vector_search_prefers_semantically_closer_entry() {
    if !configure_test_valkey() {
        return;
    }
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let storage = KnowledgeStorage::new(temp_dir.path().to_string_lossy().as_ref(), "knowledge");
    storage.init().await.expect("init should succeed");
    storage.clear().await.expect("clear should succeed");

    let e1 = crate::types::KnowledgeEntry::new(
        "vec-1".to_string(),
        "Typed language benefits".to_string(),
        "Type systems catch compile-time errors and improve refactoring safety.".to_string(),
        KnowledgeCategory::Pattern,
    );
    let e2 = crate::types::KnowledgeEntry::new(
        "vec-2".to_string(),
        "Deployment workflow".to_string(),
        "Release flow focuses on canary rollout and rollback strategy.".to_string(),
        KnowledgeCategory::Workflow,
    );

    storage.upsert(&e1).await.expect("upsert should succeed");
    storage.upsert(&e2).await.expect("upsert should succeed");

    let query = storage
        .text_to_vector("Type systems catch compile-time errors and improve refactoring safety.");
    let hits = storage
        .search(&query, 1)
        .await
        .expect("search should succeed");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "vec-1".to_string());
}
