use super::updated_repo_publication_revisions;

#[test]
fn updated_repo_publication_revisions_moves_revision_to_front_and_trims_tail() {
    let (retained, evicted) = updated_repo_publication_revisions(
        vec!["rev-2".to_string(), "rev-1".to_string()],
        "rev-3",
        2,
    );

    assert_eq!(retained, vec!["rev-3".to_string(), "rev-2".to_string()]);
    assert_eq!(evicted, vec!["rev-1".to_string()]);
}

#[test]
fn updated_repo_publication_revisions_deduplicates_existing_revision() {
    let (retained, evicted) = updated_repo_publication_revisions(
        vec!["rev-2".to_string(), "rev-1".to_string()],
        " rev-1 ",
        3,
    );

    assert_eq!(retained, vec!["rev-1".to_string(), "rev-2".to_string(),]);
    assert!(evicted.is_empty());
}
