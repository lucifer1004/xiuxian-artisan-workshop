use super::{get_setting_bool, get_setting_string, get_setting_string_list};
use serde_yaml::Value;

#[test]
fn access_helpers_read_scalar_and_sequence_values() {
    let settings: Value = serde_yaml::from_str(
        r#"
feature:
  enabled: "true"
  name: demo
  dirs:
    - src
    - tests
"#,
    )
    .unwrap_or_else(|error| panic!("yaml parse should succeed: {error}"));

    assert_eq!(
        get_setting_string(&settings, "feature.name"),
        Some("demo".to_string())
    );
    assert_eq!(get_setting_bool(&settings, "feature.enabled"), Some(true));
    assert_eq!(
        get_setting_string_list(&settings, "feature.dirs"),
        vec!["src".to_string(), "tests".to_string()]
    );
}
