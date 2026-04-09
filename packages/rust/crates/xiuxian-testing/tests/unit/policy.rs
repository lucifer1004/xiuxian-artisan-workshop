use super::*;
use std::fs;

fn create_temp_crate() -> tempfile::TempDir {
    let temp = match tempfile::tempdir() {
        Ok(temp) => temp,
        Err(error) => panic!("tempdir should be created: {error}"),
    };
    if let Err(error) = fs::create_dir_all(temp.path().join("src")) {
        panic!("src dir should be created: {error}");
    }
    write_manifest(temp.path(), "");
    temp
}

fn write_manifest(crate_root: &Path, extra: &str) {
    let manifest =
        format!("[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n{extra}");
    if let Err(error) = fs::write(crate_root.join("Cargo.toml"), manifest) {
        panic!("Cargo.toml should be written: {error}");
    }
}

fn write_fixture_file(crate_root: &Path, relative_path: &str, content: &str) {
    let path = crate_root.join(relative_path);
    let Some(parent) = path.parent() else {
        panic!("fixture path should have parent: {path:?}");
    };
    if let Err(error) = fs::create_dir_all(parent) {
        panic!("fixture directories should be created: {error}");
    }
    if let Err(error) = fs::write(path, content) {
        panic!("fixture file should be written: {error}");
    }
}

#[test]
fn validate_crate_test_policy_returns_clean_report_for_valid_crate() {
    let temp = create_temp_crate();
    write_fixture_file(
        temp.path(),
        "src/foo.rs",
        r#"
fn helper() {}

#[cfg(test)]
#[path = "../tests/unit/foo.rs"]
mod tests;
"#,
    );
    write_fixture_file(
        temp.path(),
        "tests/unit/foo.rs",
        r"
use super::*;

#[test]
fn helper_exists() {
    helper();
}
",
    );

    let report = validate_crate_test_policy(temp.path());
    assert!(report.is_clean(), "expected clean report, got {report:?}");
}

#[test]
fn validate_crate_test_policy_collects_both_policy_layers() {
    let temp = create_temp_crate();
    write_fixture_file(
        temp.path(),
        "src/foo.rs",
        r"
#[cfg(test)]
mod tests {
    #[test]
    fn inline_policy_violation() {}
}
",
    );
    write_fixture_file(
        temp.path(),
        "tests/test_foo.rs",
        "#[test]\nfn scattered() {}\n",
    );

    let report = validate_crate_test_policy(temp.path());
    assert_eq!(report.external_test_issues.len(), 1);
    assert_eq!(report.structure_violations.len(), 1);

    let formatted = format_crate_test_policy_report(&report);
    assert!(formatted.contains("External Test Policy"));
    assert!(formatted.contains("Test Structure Policy"));
}

#[test]
fn validate_crate_test_policy_with_workspace_config_applies_overrides() {
    let temp = create_temp_crate();
    write_fixture_file(
        temp.path(),
        "tests/coactivation_multihop_diffusion.rs",
        "#[test]\nfn smoke() {}\n",
    );
    write_fixture_file(
        temp.path(),
        "tests/bench/throughput.rs",
        "#[test]\nfn smoke() {}\n",
    );
    write_fixture_file(
        temp.path(),
        TEST_POLICY_CONFIG_FILE,
        r#"
[tests]
allowed_root_files = [
  { name = "coactivation_multihop_diffusion.rs", explanation = "Legacy root harness pending structured migration." },
]
allowed_directories = [
  { name = "bench", explanation = "Legacy benchmark directory pending performance harness migration." },
]
"#,
    );

    let report = validate_crate_test_policy_with_workspace_config(temp.path())
        .unwrap_or_else(|error| panic!("workspace-config validation should pass: {error}"));
    assert!(report.is_clean(), "expected clean report, got {report:?}");
}

#[test]
fn validate_crate_test_policy_with_workspace_config_rejects_invalid_toml() {
    let temp = create_temp_crate();
    write_fixture_file(
        temp.path(),
        TEST_POLICY_CONFIG_FILE,
        r#"
[tests
allowed_root_files = ["coactivation_multihop_diffusion.rs"]
"#,
    );

    let Err(error) = validate_crate_test_policy_with_workspace_config(temp.path()) else {
        panic!("invalid toml should fail");
    };
    assert!(error.contains(TEST_POLICY_CONFIG_FILE));
}

#[test]
fn validate_crate_test_policy_with_workspace_config_rejects_missing_explanation() {
    let temp = create_temp_crate();
    write_fixture_file(
        temp.path(),
        TEST_POLICY_CONFIG_FILE,
        r#"
[tests]
allowed_root_files = [
  { name = "coactivation_multihop_diffusion.rs" },
]
"#,
    );

    let Err(error) = validate_crate_test_policy_with_workspace_config(temp.path()) else {
        panic!("missing explanation should fail");
    };
    assert!(error.contains("allowed_root_files"));
    assert!(error.contains("coactivation_multihop_diffusion.rs"));
    assert!(error.contains("explanation"));
}

#[test]
fn assert_crate_tests_structure_with_workspace_config_ignores_external_layer() {
    let temp = create_temp_crate();
    write_fixture_file(
        temp.path(),
        "src/foo.rs",
        r"
#[cfg(test)]
mod tests {
    #[test]
    fn inline_policy_violation() {}
}
",
    );
    write_fixture_file(
        temp.path(),
        "tests/coactivation_weighted_propagation.rs",
        "#[test]\nfn smoke() {}\n",
    );
    write_fixture_file(
        temp.path(),
        TEST_POLICY_CONFIG_FILE,
        r#"
[tests]
allowed_root_files = [
  { name = "coactivation_weighted_propagation.rs", explanation = "Legacy root test harness kept temporarily at tests root." },
]
"#,
    );

    assert_crate_tests_structure_with_workspace_config(temp.path());
}

#[test]
fn assert_crate_test_policy_with_workspace_config_rejects_inline_test_blocks() {
    let temp = create_temp_crate();
    write_fixture_file(
        temp.path(),
        "src/foo.rs",
        r"
#[cfg(test)]
mod tests {
    #[test]
    fn inline_policy_violation() {}
}
",
    );
    write_fixture_file(
        temp.path(),
        TEST_POLICY_CONFIG_FILE,
        r"
[tests]
allowed_root_files = []
allowed_directories = []
",
    );

    let Err(panic) = std::panic::catch_unwind(|| {
        assert_crate_test_policy_with_workspace_config(temp.path());
    }) else {
        panic!("full crate test policy should reject inline cfg(test) blocks");
    };

    let message = if let Some(message) = panic.downcast_ref::<String>() {
        message.as_str()
    } else if let Some(message) = panic.downcast_ref::<&str>() {
        message
    } else {
        panic!("panic payload should be a string message");
    };

    assert!(message.contains("External Test Policy"));
    assert!(message.contains("Inline cfg(test) module"));
    assert!(message.contains("../tests/unit/foo.rs"));
}

#[test]
fn validate_crate_test_policy_harness_reports_missing_target_gate_mounts() {
    let temp = create_temp_crate();
    write_manifest(
        temp.path(),
        r#"

[[test]]
name = "runtime_config"
path = "tests/integration/runtime_config.rs"
"#,
    );
    write_fixture_file(
        temp.path(),
        "tests/integration/runtime_config.rs",
        "#[test]\nfn smoke() {}\n",
    );

    let report = validate_crate_test_policy_harness(temp.path())
        .unwrap_or_else(|error| panic!("harness validation should succeed: {error}"));
    assert!(report.policy_report.is_clean(), "{report:?}");
    assert_eq!(report.target_gate_violations.len(), 1);
    assert_eq!(
        report.target_gate_violations[0].target_file,
        PathBuf::from("tests/integration/runtime_config.rs")
    );

    let formatted = format_crate_test_policy_harness_report(&report);
    assert!(formatted.contains("Test Target Gate Policy"));
    assert!(formatted.contains("crate_test_policy_harness!"));
}

#[test]
fn validate_crate_test_policy_harness_accepts_macro_mounted_targets() {
    let temp = create_temp_crate();
    write_manifest(
        temp.path(),
        r#"

[[test]]
name = "runtime_config"
path = "tests/integration/runtime_config.rs"
"#,
    );
    write_fixture_file(
        temp.path(),
        "tests/integration/runtime_config.rs",
        r"
xiuxian_testing::crate_test_policy_harness!();

#[test]
fn smoke() {}
",
    );

    let report = validate_crate_test_policy_harness(temp.path())
        .unwrap_or_else(|error| panic!("harness validation should succeed: {error}"));
    assert!(report.is_clean(), "{report:?}");
}

#[test]
fn validate_crate_test_policy_harness_accepts_legacy_explicit_gate_target() {
    let temp = create_temp_crate();
    write_fixture_file(
        temp.path(),
        "tests/xiuxian-testing-gate.rs",
        r#"
use std::path::Path;

use xiuxian_testing::assert_crate_test_policy_with_workspace_config;

#[test]
fn enforce_gate() {
    assert_crate_test_policy_with_workspace_config(Path::new(env!("CARGO_MANIFEST_DIR")));
}
"#,
    );

    let report = validate_crate_test_policy_harness(temp.path())
        .unwrap_or_else(|error| panic!("harness validation should succeed: {error}"));
    assert!(report.is_clean(), "{report:?}");
}

#[test]
fn validate_crate_test_policy_harness_reports_missing_source_gate_mounts() {
    let temp = create_temp_crate();
    write_fixture_file(temp.path(), "src/lib.rs", "mod foo;\n");
    write_fixture_file(
        temp.path(),
        "src/foo.rs",
        r#"
fn helper() {}

#[cfg(test)]
#[path = "../tests/unit/foo.rs"]
mod tests;
"#,
    );
    write_fixture_file(
        temp.path(),
        "tests/unit/foo.rs",
        "use super::*;\n#[test]\nfn helper_exists() { helper(); }\n",
    );

    let report = validate_crate_test_policy_harness(temp.path())
        .unwrap_or_else(|error| panic!("harness validation should succeed: {error}"));
    assert!(report.policy_report.is_clean(), "{report:?}");
    assert_eq!(report.target_gate_violations.len(), 0);
    assert_eq!(report.source_gate_violations.len(), 1);
    assert_eq!(
        report.source_gate_violations[0].source_file,
        PathBuf::from("src/lib.rs")
    );

    let formatted = format_crate_test_policy_harness_report(&report);
    assert!(formatted.contains("Source Test Gate Policy"));
    assert!(formatted.contains("crate_test_policy_source_harness!"));
}

#[test]
fn validate_crate_test_policy_harness_accepts_source_harness_macro() {
    let temp = create_temp_crate();
    write_fixture_file(
        temp.path(),
        "src/lib.rs",
        r#"
xiuxian_testing::crate_test_policy_source_harness!("../tests/unit/lib_policy.rs");

mod foo;
"#,
    );
    write_fixture_file(
        temp.path(),
        "src/foo.rs",
        r#"
fn helper() {}

#[cfg(test)]
#[path = "../tests/unit/foo.rs"]
mod tests;
"#,
    );
    write_fixture_file(
        temp.path(),
        "tests/unit/foo.rs",
        "use super::*;\n#[test]\nfn helper_exists() { helper(); }\n",
    );
    write_fixture_file(
        temp.path(),
        "tests/unit/lib_policy.rs",
        "xiuxian_testing::crate_test_policy_harness!();\n",
    );

    let report = validate_crate_test_policy_harness(temp.path())
        .unwrap_or_else(|error| panic!("harness validation should succeed: {error}"));
    assert!(report.is_clean(), "{report:?}");
}
