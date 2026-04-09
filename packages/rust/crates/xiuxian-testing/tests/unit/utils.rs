use super::*;
use serde_json::json;

#[test]
fn test_assert_json_eq_success() {
    let a = json!({"key": "value"});
    let b = json!({"key": "value"});
    assert_json_eq(&a, &b);
}

#[test]
fn test_temp_dir_with_prefix() {
    let dir = temp_dir_with_prefix("test_");
    assert!(dir.path().exists());
}
