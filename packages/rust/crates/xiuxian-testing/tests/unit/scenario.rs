use super::*;
use std::fs;

fn write_scenario_fixture(root: &Path, name: &str, category: &str) {
    write_scenario_fixture_with_id(root, name, name, category);
}

fn write_scenario_fixture_with_id(root: &Path, dir_name: &str, id: &str, category: &str) {
    let scenario_dir = root.join(dir_name);
    if let Err(error) = fs::create_dir_all(&scenario_dir) {
        panic!("scenario dir should be created: {error}");
    }
    if let Err(error) = fs::write(
        scenario_dir.join("scenario.toml"),
        format!(
            r#"[scenario]
id = "{id}"
name = "Fixture Scenario"
description = "Fixture"
category = "{category}"

[input]
type = "json"
"#
        ),
    ) {
        panic!("scenario.toml should be written: {error}");
    }
}

#[test]
fn test_framework_new() {
    let framework = ScenarioFramework::new();
    assert!(framework.find_runner("nonexistent").is_none());
    assert!(framework.snapshot_policy().sort_maps());
    assert!(!framework.snapshot_policy().include_description());
    assert!(!framework.snapshot_policy().include_info());
}

#[test]
fn test_scenario_config_default() {
    let config = RunnerConfig::default();
    assert!(config.build_page_index.is_none());
    assert!(config.collect_links.is_none());
}

#[test]
fn test_scenarios_root_returns_crate_local() {
    let root = scenarios_root();
    assert!(
        root.ends_with("tests/scenarios"),
        "scenarios_root should end with tests/scenarios: {root:?}"
    );
}

#[test]
fn run_all_at_fails_when_scenario_ids_collide() {
    let temp = match tempfile::tempdir() {
        Ok(temp) => temp,
        Err(error) => panic!("tempdir should be created: {error}"),
    };
    write_scenario_fixture_with_id(temp.path(), "001_first", "001_collision", "collision_case");
    write_scenario_fixture_with_id(temp.path(), "002_second", "001_collision", "collision_case");

    let framework = ScenarioFramework::new();
    let Err(error) = framework.run_all_at(temp.path()) else {
        panic!("duplicate scenario ids should fail closed");
    };
    let message = error.to_string();

    assert!(
        message.contains("Duplicate scenario id '001_collision'"),
        "unexpected error: {error}"
    );
    assert!(
        message.contains("001_first") && message.contains("002_second"),
        "duplicate path context should be present: {error}"
    );
}

#[test]
fn test_discover_scenarios_returns_local() {
    let scenarios = discover_scenarios();
    for scenario in &scenarios {
        assert!(
            scenario.to_string_lossy().contains("tests/scenarios"),
            "Scenario should be in crate-local path: {scenario:?}"
        );
    }
}

#[test]
fn run_all_at_fails_when_scenario_has_no_registered_runner() {
    let temp = match tempfile::tempdir() {
        Ok(temp) => temp,
        Err(error) => panic!("tempdir should be created: {error}"),
    };
    write_scenario_fixture(temp.path(), "001_missing_runner", "missing_runner");

    let framework = ScenarioFramework::new();
    let Err(error) = framework.run_all_at(temp.path()) else {
        panic!("missing runner should fail closed");
    };

    assert!(
        error
            .to_string()
            .contains("No runner registered for scenario category 'missing_runner'"),
        "unexpected error: {error}"
    );
}

#[test]
fn snapshot_policy_recommended_enables_rich_metadata() {
    let policy = ScenarioSnapshotPolicy::recommended();

    assert!(policy.sort_maps());
    assert!(policy.include_description());
    assert!(policy.include_info());
    assert!(policy.include_input_file());
    assert!(
        policy
            .redactions()
            .contains(&ScenarioSnapshotRedaction::normalize_path(".**.path"))
    );
    assert!(
        policy
            .redactions()
            .contains(&ScenarioSnapshotRedaction::replace(
                ".**.request_id",
                "[request-id]"
            ))
    );
    assert!(
        !policy
            .redactions()
            .contains(&ScenarioSnapshotRedaction::round(".**.latency_ms", 2))
    );
}

#[test]
fn snapshot_policy_portable_ci_disables_input_file_metadata() {
    let policy = ScenarioSnapshotPolicy::portable_ci();

    assert!(policy.sort_maps());
    assert!(policy.include_description());
    assert!(policy.include_info());
    assert!(!policy.include_input_file());
}

#[test]
fn snapshot_policy_runtime_heavy_adds_timing_redactions() {
    let policy = ScenarioSnapshotPolicy::runtime_heavy();

    assert!(policy.include_input_file());
    assert!(
        policy
            .redactions()
            .contains(&ScenarioSnapshotRedaction::round(".**.latency_ms", 2))
    );
    assert!(
        policy
            .redactions()
            .contains(&ScenarioSnapshotRedaction::round(".**.duration_secs", 4))
    );
}

#[test]
fn snapshot_policy_supports_redaction_builders() {
    let mut policy = ScenarioSnapshotPolicy::new();
    policy
        .add_redaction(ScenarioSnapshotRedaction::replace(".request.id", "[id]"))
        .add_redaction(ScenarioSnapshotRedaction::sort(".flags"))
        .add_redaction(ScenarioSnapshotRedaction::round(".timings.latency_ms", 2))
        .add_redaction(ScenarioSnapshotRedaction::normalize_path(
            ".artifacts.output_path",
        ));

    assert_eq!(policy.redactions().len(), 4);
    assert_eq!(
        policy.redactions()[0],
        ScenarioSnapshotRedaction::Replace {
            selector: ".request.id".to_string(),
            replacement: "[id]".to_string(),
        }
    );
    assert_eq!(
        policy.redactions()[1],
        ScenarioSnapshotRedaction::Sort {
            selector: ".flags".to_string(),
        }
    );
    assert_eq!(
        policy.redactions()[2],
        ScenarioSnapshotRedaction::Round {
            selector: ".timings.latency_ms".to_string(),
            decimals: 2,
        }
    );
    assert_eq!(
        policy.redactions()[3],
        ScenarioSnapshotRedaction::NormalizePath {
            selector: ".artifacts.output_path".to_string(),
        }
    );
}

#[test]
fn snapshot_policy_supports_redaction_presets() {
    let mut policy = ScenarioSnapshotPolicy::new();
    policy
        .add_redaction_preset(ScenarioSnapshotRedactionPreset::portable_paths())
        .add_redaction_preset(ScenarioSnapshotRedactionPreset::runtime_volatility())
        .add_redaction_preset(ScenarioSnapshotRedactionPreset::timing_noise());

    assert!(
        policy
            .redactions()
            .contains(&ScenarioSnapshotRedaction::normalize_path(".**.temp_dir"))
    );
    assert!(
        policy
            .redactions()
            .contains(&ScenarioSnapshotRedaction::replace(
                ".**.started_at",
                "[started-at]"
            ))
    );
    assert!(
        policy
            .redactions()
            .contains(&ScenarioSnapshotRedaction::round(".**.elapsed_ms", 2))
    );
}

#[test]
fn snapshot_policy_builds_metadata_rich_settings() {
    let temp = match tempfile::tempdir() {
        Ok(temp) => temp,
        Err(error) => panic!("tempdir should be created: {error}"),
    };
    write_scenario_fixture(temp.path(), "001_policy", "policy_case");
    let scenario = match Scenario::load(temp.path().join("001_policy")) {
        Ok(scenario) => scenario,
        Err(error) => panic!("scenario should load: {error}"),
    };
    let snapshot_path = temp.path().join("snapshots");
    let policy = ScenarioSnapshotPolicy::recommended();

    let settings = policy.settings_for(&snapshot_path, &scenario);

    assert!(settings.sort_maps());
    assert!(!settings.prepend_module_to_snapshot());
    assert_eq!(settings.snapshot_path(), snapshot_path.as_path());
    assert_eq!(
        settings.description(),
        Some("Scenario 001_policy [policy_case]: Fixture Scenario")
    );
    assert_eq!(
        settings.input_file(),
        Some(scenario.dir.join("scenario.toml").as_path())
    );
    assert!(settings.has_info());
}

#[test]
fn snapshot_policy_portable_ci_omits_input_file_from_settings() {
    let temp = match tempfile::tempdir() {
        Ok(temp) => temp,
        Err(error) => panic!("tempdir should be created: {error}"),
    };
    write_scenario_fixture(temp.path(), "001_portable", "portable_case");
    let scenario = match Scenario::load(temp.path().join("001_portable")) {
        Ok(scenario) => scenario,
        Err(error) => panic!("scenario should load: {error}"),
    };
    let snapshot_path = temp.path().join("snapshots");
    let policy = ScenarioSnapshotPolicy::portable_ci();

    let settings = policy.settings_for(&snapshot_path, &scenario);

    assert_eq!(settings.input_file(), None);
    assert!(settings.has_info());
    assert_eq!(
        settings.description(),
        Some("Scenario 001_portable [portable_case]: Fixture Scenario")
    );
}

#[test]
fn snapshot_policy_clears_disabled_metadata_from_parent_settings() {
    let temp = match tempfile::tempdir() {
        Ok(temp) => temp,
        Err(error) => panic!("tempdir should be created: {error}"),
    };
    write_scenario_fixture(temp.path(), "001_clean", "clean_case");
    let scenario = match Scenario::load(temp.path().join("001_clean")) {
        Ok(scenario) => scenario,
        Err(error) => panic!("scenario should load: {error}"),
    };
    let snapshot_path = temp.path().join("snapshots");
    let policy = ScenarioSnapshotPolicy::portable_ci();
    let mut parent = insta::Settings::new();
    parent.set_description("parent description");
    parent.set_input_file(temp.path().join("parent.toml"));
    parent.set_info(&serde_json::json!({ "parent": true }));

    parent.bind(|| {
        let settings = policy.settings_for(&snapshot_path, &scenario);

        assert_eq!(settings.input_file(), None);
        assert_eq!(
            settings.description(),
            Some("Scenario 001_clean [clean_case]: Fixture Scenario")
        );
        assert!(settings.has_info());
    });
}

#[test]
fn snapshot_policy_new_removes_parent_metadata_when_disabled() {
    let temp = match tempfile::tempdir() {
        Ok(temp) => temp,
        Err(error) => panic!("tempdir should be created: {error}"),
    };
    write_scenario_fixture(temp.path(), "001_minimal", "minimal_case");
    let scenario = match Scenario::load(temp.path().join("001_minimal")) {
        Ok(scenario) => scenario,
        Err(error) => panic!("scenario should load: {error}"),
    };
    let snapshot_path = temp.path().join("snapshots");
    let policy = ScenarioSnapshotPolicy::new();
    let mut parent = insta::Settings::new();
    parent.set_description("parent description");
    parent.set_input_file(temp.path().join("parent.toml"));
    parent.set_info(&serde_json::json!({ "parent": true }));

    parent.bind(|| {
        let settings = policy.settings_for(&snapshot_path, &scenario);

        assert_eq!(settings.description(), None);
        assert_eq!(settings.input_file(), None);
        assert!(!settings.has_info());
    });
}

#[test]
fn framework_can_replace_snapshot_policy() {
    let mut framework = ScenarioFramework::new();
    let mut policy = ScenarioSnapshotPolicy::recommended();
    policy.set_sort_maps(false);
    framework.set_snapshot_policy(policy.clone());

    assert_eq!(framework.snapshot_policy(), &policy);
    assert!(!framework.snapshot_policy().sort_maps());
}

#[test]
fn normalize_path_redaction_stabilizes_workspace_and_temp_prefixes() {
    let Some(workspace_root) = workspace_root() else {
        panic!("workspace root should be detected");
    };
    let workspace_path = workspace_root
        .join("packages")
        .join("rust")
        .join("crates")
        .join("xiuxian-testing")
        .to_string_lossy()
        .replace('/', "\\");
    let temp_path = std::env::temp_dir()
        .join("xiuxian-testing")
        .join("fixture.json")
        .to_string_lossy()
        .to_string();
    let mut settings = insta::Settings::new();
    ScenarioSnapshotRedaction::normalize_path(".workspace").apply(&mut settings);
    ScenarioSnapshotRedaction::normalize_path(".temp").apply(&mut settings);

    settings.bind(|| {
        insta::assert_json_snapshot!(
            serde_json::json!({
                "workspace": workspace_path,
                "temp": temp_path,
                "relative": "docs/alpha.md",
            }),
            @r#"
            {
              "relative": "docs/alpha.md",
              "temp": "[temp]/xiuxian-testing/fixture.json",
              "workspace": "[workspace]/packages/rust/crates/xiuxian-testing"
            }
            "#
        );
    });
}
