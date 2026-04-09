use super::normalize_relative_path;

#[test]
fn normalize_relative_path_trims_prefix_and_separators() {
    assert_eq!(
        normalize_relative_path(" ./references\\\\qianji.toml "),
        "references/qianji.toml".to_string()
    );
}
