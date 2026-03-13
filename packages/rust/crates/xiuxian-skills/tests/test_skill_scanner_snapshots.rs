//! Snapshot contracts for `SkillScanner` metadata and strict validation outputs.
//!
//! Uses Insta for snapshot testing.

mod support;

use std::fs;

use support::{
    canonicalize_json, default_structure, read_fixture, sanitize_json_paths, sanitize_path,
    write_fixture_file,
};
use tempfile::TempDir;
use xiuxian_skills::{SkillMetadata, SkillScanner, ToolAnnotations, ToolRecord};

// ============================================================================
// Snapshot: Skill Metadata Parse
// ============================================================================

#[test]
fn snapshot_skill_metadata_parse_contract() {
    let scanner = SkillScanner::new();
    let temp_dir = TempDir::new().expect("temp dir");
    let skill_path = temp_dir.path().join("auditor_neuron");
    fs::create_dir_all(&skill_path).expect("create dir");
    let content = read_fixture("skill_scanner_snapshots/auditor_neuron_parse/SKILL.md");

    let metadata = scanner
        .parse_skill_md(content.as_str(), &skill_path)
        .expect("parse");
    insta::assert_json_snapshot!("parsed_metadata", metadata);
}

// ============================================================================
// Snapshot: Structure Validation Summary
// ============================================================================

#[test]
fn snapshot_structure_validation_summary_contract() {
    let temp_dir = TempDir::new().expect("temp dir");
    let skill_path = temp_dir.path().join("auditor_neuron");
    fs::create_dir_all(skill_path.join("scripts")).expect("create scripts");
    fs::create_dir_all(skill_path.join("references")).expect("create references");
    fs::create_dir_all(skill_path.join("scratch")).expect("create scratch");
    write_fixture_file(
        skill_path.join("SKILL.md").as_path(),
        "skill_scanner_snapshots/auditor_neuron_base/SKILL.md",
    );

    let structure = default_structure();
    let report = SkillScanner::validate_structure_report(&skill_path, &structure);

    // Use the report fields directly
    let summary = serde_json::json!({
        "valid": report.valid,
        "issues": report.issues,
    });

    insta::assert_json_snapshot!("structure_report_summary", summary);
}

// ============================================================================
// Snapshot: Missing Type Error
// ============================================================================

#[test]
fn snapshot_missing_type_error_contract() {
    let temp_dir = TempDir::new().expect("temp dir");
    let skill_path = temp_dir.path().join("auditor_neuron");
    fs::create_dir_all(skill_path.join("references")).expect("create references");
    write_fixture_file(
        skill_path.join("SKILL.md").as_path(),
        "skill_scanner_snapshots/auditor_neuron_base/SKILL.md",
    );
    write_fixture_file(
        skill_path.join("references/teacher.md").as_path(),
        "skill_scanner_snapshots/missing_type/teacher.md",
    );

    let scanner = SkillScanner::new();
    let structure = default_structure();
    let error = scanner
        .scan_skill(&skill_path, Some(&structure))
        .err()
        .expect("expected error");

    let normalized = sanitize_path(&error.to_string(), &skill_path);
    insta::assert_snapshot!("missing_type_error", normalized);
}

// ============================================================================
// Snapshot: Structure Validation Issues
// ============================================================================

#[test]
fn snapshot_structure_validation_issues_contract() {
    let temp_dir = TempDir::new().expect("temp dir");
    let skill_path = temp_dir.path().join("auditor_neuron");
    fs::create_dir_all(skill_path.join("scripts")).expect("create scripts");
    fs::create_dir_all(skill_path.join("references")).expect("create references");
    fs::create_dir_all(skill_path.join("scratch")).expect("create scratch");
    write_fixture_file(
        skill_path.join("SKILL.md").as_path(),
        "skill_scanner_snapshots/auditor_neuron_base/SKILL.md",
    );

    let structure = default_structure();
    let report = SkillScanner::validate_structure_report(&skill_path, &structure);

    // Sanitize paths in issues
    let sanitized_issues: Vec<String> = report
        .issues
        .iter()
        .map(|issue| sanitize_path(issue, &skill_path))
        .collect();

    insta::assert_json_snapshot!("structure_report_issues", sanitized_issues);
}

// ============================================================================
// Snapshot: Scan All Multiple Skills
// ============================================================================

#[test]
fn snapshot_scan_all_multiple_skills_contract() {
    let temp_dir = TempDir::new().expect("temp dir");
    let skills_dir = temp_dir.path().join("skills");
    fs::create_dir_all(&skills_dir).expect("create skills dir");

    let writer_path = skills_dir.join("writer");
    fs::create_dir_all(&writer_path).expect("create writer");
    write_fixture_file(
        writer_path.join("SKILL.md").as_path(),
        "skill_scanner_snapshots/scan_all/writer/SKILL.md",
    );

    let git_path = skills_dir.join("git");
    fs::create_dir_all(&git_path).expect("create git");
    write_fixture_file(
        git_path.join("SKILL.md").as_path(),
        "skill_scanner_snapshots/scan_all/git/SKILL.md",
    );

    let scanner = SkillScanner::new();
    let mut metadatas = scanner.scan_all(&skills_dir, None).expect("scan all");
    metadatas.sort_by(|left, right| left.skill_name.cmp(&right.skill_name));

    insta::assert_json_snapshot!("scan_all_multiple_skills", metadatas);
}

// ============================================================================
// Snapshot: Canonical Payload Tool Reference
// ============================================================================

#[test]
fn snapshot_canonical_payload_tool_reference_contract() {
    let temp_dir = TempDir::new().expect("temp dir");
    let skill_path = temp_dir.path().join("researcher");
    fs::create_dir_all(&skill_path).expect("create researcher");
    fs::create_dir_all(skill_path.join("references")).expect("create references");
    write_fixture_file(
        skill_path.join("SKILL.md").as_path(),
        "skill_scanner_snapshots/canonical_payload/researcher/SKILL.md",
    );
    write_fixture_file(
        skill_path
            .join("references/run_research_graph.md")
            .as_path(),
        "skill_scanner_snapshots/canonical_payload/researcher/references/run_research_graph.md",
    );

    let metadata = SkillMetadata {
        skill_name: "researcher".to_string(),
        version: "1.0.0".to_string(),
        description: "Research skill".to_string(),
        routing_keywords: vec!["research".to_string()],
        authors: vec![],
        intents: vec![],
        require_refs: vec![],
        repository: String::new(),
        permissions: vec![],
    };

    let tools = vec![ToolRecord {
        tool_name: "researcher.run_research_graph".to_string(),
        description: "Run the graph".to_string(),
        skill_name: "researcher".to_string(),
        file_path: "researcher/scripts/commands.py".to_string(),
        function_name: "run_research_graph".to_string(),
        execution_mode: "async".to_string(),
        keywords: vec!["research".to_string()],
        intents: vec![],
        file_hash: "abc".to_string(),
        input_schema: "{}".to_string(),
        docstring: String::new(),
        category: "research".to_string(),
        annotations: ToolAnnotations::default(),
        parameters: vec![],
        skill_tools_refers: vec![],
        resource_uri: String::new(),
    }];

    let scanner = SkillScanner::new();
    let payload = scanner.build_canonical_payload(metadata, &tools, &skill_path);
    let mut json_value = serde_json::to_value(payload).expect("to json");
    sanitize_json_paths(&mut json_value, &skill_path);
    let canonical = canonicalize_json(json_value);

    insta::assert_json_snapshot!("canonical_payload_tool_reference", canonical);
}
