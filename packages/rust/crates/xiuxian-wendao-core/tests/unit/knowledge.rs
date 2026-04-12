use xiuxian_wendao_core::{KnowledgeCategory, KnowledgeEntry};

#[test]
fn knowledge_entry_new_sets_expected_defaults() {
    let entry = KnowledgeEntry::new(
        "entry-1".to_string(),
        "Contract finding".to_string(),
        "Knowledge body".to_string(),
        KnowledgeCategory::Reference,
    );

    assert_eq!(entry.id, "entry-1");
    assert_eq!(entry.title, "Contract finding");
    assert_eq!(entry.content, "Knowledge body");
    assert_eq!(entry.category, KnowledgeCategory::Reference);
    assert!(entry.tags.is_empty());
    assert_eq!(entry.source, None);
    assert_eq!(entry.version, 1);
    assert!(entry.metadata.is_empty());
}

#[test]
fn knowledge_entry_add_tag_deduplicates_values() {
    let mut entry = KnowledgeEntry::new(
        "entry-2".to_string(),
        "Note".to_string(),
        "Body".to_string(),
        KnowledgeCategory::Note,
    );

    entry.add_tag("alpha".to_string());
    entry.add_tag("alpha".to_string());
    entry.add_tag("beta".to_string());

    assert_eq!(entry.tags, vec!["alpha".to_string(), "beta".to_string()]);
}
