use super::resolve_link_graph_related_runtime_with_settings;
use crate::config::test_support;
use std::fs;

#[test]
fn resolve_related_runtime_reads_override_values() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r"[link_graph.related]
max_candidates = 512
max_partitions = 4
time_budget_ms = 75.0
",
    )?;

    let settings = test_support::load_test_settings_from_path(&config_path)?;
    let runtime = resolve_link_graph_related_runtime_with_settings(&settings);
    assert_eq!(runtime.max_candidates, 512);
    assert_eq!(runtime.max_partitions, 4);
    assert!((runtime.time_budget_ms - 75.0).abs() <= f64::EPSILON);

    Ok(())
}
