use std::collections::BTreeSet;

use super::{DELETE_PATH_FILTER_BATCH_SIZE, path_delete_filters};

#[test]
fn path_delete_filters_returns_empty_for_empty_sets() {
    assert!(path_delete_filters("path", &BTreeSet::new()).is_empty());
}

#[test]
fn path_delete_filters_escapes_quotes() {
    let filters = path_delete_filters("path", &BTreeSet::from(["dir/o'clock.rs".to_string()]));
    assert_eq!(filters, vec!["path IN ('dir/o''clock.rs')".to_string()]);
}

#[test]
fn path_delete_filters_batches_large_sets() {
    let paths = (0..=(DELETE_PATH_FILTER_BATCH_SIZE * 2))
        .map(|index| format!("src/file_{index:03}.rs"))
        .collect::<BTreeSet<_>>();
    let filters = path_delete_filters("path", &paths);

    assert_eq!(filters.len(), 3);
    assert!(filters[0].starts_with("path IN ("));
    assert!(filters[1].starts_with("path IN ("));
    assert!(filters[2].starts_with("path IN ("));
    assert!(filters[0].contains("src/file_000.rs"));
    assert!(filters[1].contains(&format!("src/file_{DELETE_PATH_FILTER_BATCH_SIZE:03}.rs")));
    assert!(filters[2].contains(&format!(
        "src/file_{:03}.rs",
        DELETE_PATH_FILTER_BATCH_SIZE * 2
    )));
}
