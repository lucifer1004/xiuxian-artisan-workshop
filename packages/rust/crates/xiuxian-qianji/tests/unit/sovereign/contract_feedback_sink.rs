use super::{
    ContractFeedbackKnowledgeSink, InMemoryContractFeedbackSink,
    KnowledgeStorageContractFeedbackSink,
};
use xiuxian_wendao_core::{KnowledgeCategory, KnowledgeEntry};

fn test_entry(id: &str) -> KnowledgeEntry {
    KnowledgeEntry::new(
        id.to_string(),
        "Contract finding".to_string(),
        "Knowledge body".to_string(),
        KnowledgeCategory::Reference,
    )
}

fn must_ok<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> T {
    result.unwrap_or_else(|error| panic!("{context}: {error}"))
}

#[test]
fn knowledge_storage_contract_feedback_sink_exposes_configuration() {
    let sink = KnowledgeStorageContractFeedbackSink::new(".cache/wendao", "knowledge");
    assert_eq!(sink.storage_path(), ".cache/wendao");
    assert_eq!(sink.table_name(), "knowledge");
}

#[tokio::test]
async fn in_memory_contract_feedback_sink_persists_entries() {
    let sink = InMemoryContractFeedbackSink::new();
    let persisted_ids = must_ok(
        sink.persist_entries(&[test_entry("entry-1"), test_entry("entry-2")])
            .await,
        "in-memory sink should persist entries",
    );

    assert_eq!(
        persisted_ids,
        vec!["entry-1".to_string(), "entry-2".to_string()]
    );
    assert_eq!(sink.len(), 2);
    assert!(!sink.is_empty());
    assert_eq!(
        sink.entries()
            .into_iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>(),
        persisted_ids
    );
}
