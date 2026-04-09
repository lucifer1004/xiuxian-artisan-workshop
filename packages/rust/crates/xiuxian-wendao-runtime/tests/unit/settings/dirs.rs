use super::{dedup_dirs, normalize_relative_dir};

#[test]
fn dir_helpers_normalize_and_deduplicate() {
    assert_eq!(normalize_relative_dir(" /src/ "), Some("src".to_string()));
    assert_eq!(normalize_relative_dir("."), None);
    assert_eq!(
        dedup_dirs(vec![
            "src".to_string(),
            "SRC".to_string(),
            "tests".to_string(),
        ]),
        vec!["src".to_string(), "tests".to_string()]
    );
}
