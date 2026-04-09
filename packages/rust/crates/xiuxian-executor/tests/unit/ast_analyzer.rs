//! Tests for the executor AST analyzer.

use xiuxian_executor::{AstCommandAnalyzer, ViolationSeverity};

#[test]
fn analyze_detects_command_name_and_mutation_intent() {
    let analyzer = AstCommandAnalyzer::new();
    let result = analyzer.analyze("mkdir ./workspace");

    assert_eq!(result.command_name.as_deref(), Some("mkdir"));
    assert!(result.is_mutation);
}

#[test]
fn analyze_marks_variable_expansions_as_tainted() {
    let analyzer = AstCommandAnalyzer::new();
    let result = analyzer.analyze("echo $HOME");

    assert!(
        result
            .variables
            .iter()
            .any(|variable| variable.is_tainted && variable.name.contains("HOME"))
    );
}

#[test]
fn analyze_blocks_rm_rf_root_pattern() {
    let analyzer = AstCommandAnalyzer::new();
    let result = analyzer.analyze("rm -rf /");

    assert!(!result.is_safe);
    assert!(
        result
            .violations
            .iter()
            .any(|violation| violation.severity == ViolationSeverity::Blocked)
    );
}
