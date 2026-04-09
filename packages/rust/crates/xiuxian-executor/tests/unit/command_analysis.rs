//! Tests for command analysis serialization and types.

use std::path::PathBuf;

use xiuxian_executor::{
    AstCommandAnalyzer, CommandAnalysis, SecurityViolation, VariableInfo, ViolationSeverity,
};

#[test]
fn command_analysis_serializes_to_json() -> Result<(), Box<dyn std::error::Error>> {
    let analysis = CommandAnalysis {
        is_safe: true,
        is_mutation: false,
        variables: vec![VariableInfo {
            name: "TEST".to_string(),
            value: Some("value".to_string()),
            is_tainted: false,
        }],
        file_paths: vec![PathBuf::from("/tmp/test")],
        command_name: Some("echo".to_string()),
        violations: vec![SecurityViolation {
            severity: ViolationSeverity::Warning,
            rule: "TEST_RULE".to_string(),
            message: "Test message".to_string(),
            node_kind: "test".to_string(),
        }],
        fingerprint: "abc123".to_string(),
    };

    let json = serde_json::to_string(&analysis)?;
    assert!(json.contains("is_safe"));
    assert!(json.contains("TEST"));
    assert!(json.contains("/tmp/test"));
    Ok(())
}

#[test]
fn violation_severity_levels_serialize_distinctly() -> Result<(), Box<dyn std::error::Error>> {
    let blocked = SecurityViolation {
        severity: ViolationSeverity::Blocked,
        rule: "DANGEROUS".to_string(),
        message: "Blocked".to_string(),
        node_kind: "test".to_string(),
    };
    let warning = SecurityViolation {
        severity: ViolationSeverity::Warning,
        rule: "WARNING".to_string(),
        message: "Warning".to_string(),
        node_kind: "test".to_string(),
    };
    let info = SecurityViolation {
        severity: ViolationSeverity::Info,
        rule: "INFO".to_string(),
        message: "Info".to_string(),
        node_kind: "test".to_string(),
    };

    assert!(serde_json::to_string(&blocked)?.contains("Blocked"));
    assert!(serde_json::to_string(&warning)?.contains("Warning"));
    assert!(serde_json::to_string(&info)?.contains("Info"));
    Ok(())
}

#[test]
fn variable_info_tainted_flag_is_preserved() {
    let normal_var = VariableInfo {
        name: "NORMAL".to_string(),
        value: Some("value".to_string()),
        is_tainted: false,
    };
    let tainted_var = VariableInfo {
        name: "$DANGER".to_string(),
        value: None,
        is_tainted: true,
    };

    assert!(!normal_var.is_tainted);
    assert!(tainted_var.is_tainted);
}

#[test]
fn analyzer_returns_empty_collections_for_true_command() {
    let analyzer = AstCommandAnalyzer::new();
    let result = analyzer.analyze("true");

    assert!(result.is_safe);
    assert!(!result.is_mutation);
    assert!(result.variables.is_empty());
    assert!(result.file_paths.is_empty());
}

#[test]
fn analysis_fingerprint_is_hex() {
    let analyzer = AstCommandAnalyzer::new();
    let result = analyzer.analyze("ls");

    assert!(!result.fingerprint.is_empty());
    assert!(result.fingerprint.chars().all(|ch| ch.is_ascii_hexdigit()));
}
