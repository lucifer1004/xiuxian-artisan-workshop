use crate::AgendaEntry;

#[test]
fn test_new_entry_is_fresh() {
    let entry = AgendaEntry::new("Master the Wind Sword".to_string());
    assert_eq!(entry.heat, 0.5);
    assert_eq!(entry.carryover_count, 0);
    assert!(!entry.reminded);
}
