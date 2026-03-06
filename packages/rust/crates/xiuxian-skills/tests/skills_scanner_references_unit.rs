//! Integration tests for reference scanning behavior via public `SkillScanner` APIs.

use std::fs;
use std::io;
use std::path::Path;

use tempfile::TempDir;
use xiuxian_skills::{SkillMetadata, SkillScanner};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn write_skill_md(skill_path: &Path, skill_name: &str) -> TestResult {
    fs::write(
        skill_path.join("SKILL.md"),
        format!(
            r#"---
name: "{skill_name}"
metadata:
  version: "1.0"
---
# {skill_name}
"#
        ),
    )?;
    Ok(())
}

fn build_index_entry_for_skill(
    scanner: &SkillScanner,
    skill_path: &Path,
    skill_name: &str,
) -> xiuxian_skills::SkillIndexEntry {
    let metadata = SkillMetadata::with_name(skill_name);
    scanner.build_index_entry(metadata, &[], skill_path)
}

#[test]
fn test_build_index_entry_reference_record_from_frontmatter() -> TestResult {
    let temp_dir = TempDir::new()?;
    let skill_path = temp_dir.path().join("researcher");
    let references_path = skill_path.join("references");
    fs::create_dir_all(&references_path)?;
    write_skill_md(&skill_path, "researcher")?;

    let reference_doc = references_path.join("run_research_graph.md");
    fs::write(
        &reference_doc,
        r#"---
type: knowledge
metadata:
  for_tools:
    - researcher.run_research_graph
  title: "Run Research Graph"
  routing_keywords: ["research", "graph"]
  intents: ["search_docs"]
---
# body
"#,
    )?;

    let scanner = SkillScanner::new();
    let entry = build_index_entry_for_skill(&scanner, &skill_path, "researcher");
    assert_eq!(entry.references.len(), 1);

    let record = entry
        .references
        .first()
        .ok_or_else(|| io::Error::other("expected one reference record"))?;
    assert_eq!(record.ref_name, "run_research_graph");
    assert_eq!(record.title, "Run Research Graph");
    assert_eq!(record.skill_name, "researcher");
    assert_eq!(record.for_skills, vec!["researcher".to_string()]);
    assert_eq!(
        record.for_tools,
        Some(vec!["researcher.run_research_graph".to_string()])
    );
    assert_eq!(
        record.keywords,
        vec![
            "research".to_string(),
            "graph".to_string(),
            "search_docs".to_string()
        ]
    );
    assert_eq!(
        record.file_path,
        reference_doc.to_string_lossy().to_string()
    );

    Ok(())
}

#[test]
fn test_build_index_entry_reference_for_tools_scalar_and_sequence() -> TestResult {
    let temp_dir = TempDir::new()?;
    let skill_path = temp_dir.path().join("researcher");
    let references_path = skill_path.join("references");
    fs::create_dir_all(&references_path)?;
    write_skill_md(&skill_path, "researcher")?;

    fs::write(
        references_path.join("scalar.md"),
        r#"---
type: knowledge
metadata:
  title: "Scalar"
  for_tools: "researcher.run"
---
# body
"#,
    )?;
    fs::write(
        references_path.join("sequence.md"),
        r#"---
type: knowledge
metadata:
  title: "Sequence"
  for_tools:
    - researcher.run
    - writer.polish
---
# body
"#,
    )?;

    let scanner = SkillScanner::new();
    let entry = build_index_entry_for_skill(&scanner, &skill_path, "researcher");

    let scalar = entry
        .references
        .iter()
        .find(|record| record.ref_name == "scalar")
        .ok_or_else(|| io::Error::other("expected scalar record"))?;
    assert_eq!(scalar.for_tools, Some(vec!["researcher.run".to_string()]));

    let sequence = entry
        .references
        .iter()
        .find(|record| record.ref_name == "sequence")
        .ok_or_else(|| io::Error::other("expected sequence record"))?;
    assert_eq!(
        sequence.for_tools,
        Some(vec![
            "researcher.run".to_string(),
            "writer.polish".to_string()
        ])
    );

    Ok(())
}

#[test]
fn test_build_index_entry_reference_for_skills_unique_and_sorted() -> TestResult {
    let temp_dir = TempDir::new()?;
    let skill_path = temp_dir.path().join("researcher");
    let references_path = skill_path.join("references");
    fs::create_dir_all(&references_path)?;
    write_skill_md(&skill_path, "researcher")?;

    fs::write(
        references_path.join("tools.md"),
        r#"---
type: knowledge
metadata:
  title: "Tools"
  for_tools:
    - researcher.run_research_graph
    - writer.polish
    - researcher.collect
---
# body
"#,
    )?;

    let scanner = SkillScanner::new();
    let entry = build_index_entry_for_skill(&scanner, &skill_path, "researcher");
    let record = entry
        .references
        .iter()
        .find(|record| record.ref_name == "tools")
        .ok_or_else(|| io::Error::other("expected tools record"))?;

    assert_eq!(
        record.for_skills,
        vec!["researcher".to_string(), "writer".to_string()]
    );

    Ok(())
}

#[test]
fn test_scan_skill_rejects_reference_missing_type_strictly() -> TestResult {
    let temp_dir = TempDir::new()?;
    let skill_path = temp_dir.path().join("researcher");
    let references_path = skill_path.join("references");
    fs::create_dir_all(&references_path)?;
    write_skill_md(&skill_path, "researcher")?;

    fs::write(
        references_path.join("run_research_graph.md"),
        r#"---
metadata:
  title: "Run Research Graph"
---
# body
"#,
    )?;

    let scanner = SkillScanner::new();
    let error = scanner
        .scan_skill(&skill_path, None)
        .err()
        .ok_or_else(|| io::Error::other("expected strict metadata validation error"))?;
    assert!(
        error.to_string().contains("missing field `type`"),
        "unexpected error: {error}"
    );

    Ok(())
}

#[test]
fn test_scan_skill_rejects_persona_without_role_class_strictly() -> TestResult {
    let temp_dir = TempDir::new()?;
    let skill_path = temp_dir.path().join("researcher");
    let references_path = skill_path.join("references");
    fs::create_dir_all(&references_path)?;
    write_skill_md(&skill_path, "researcher")?;

    fs::write(
        references_path.join("teacher.md"),
        r#"---
type: persona
metadata:
  title: "Strict Teacher"
---
# body
"#,
    )?;

    let scanner = SkillScanner::new();
    let error = scanner
        .scan_skill(&skill_path, None)
        .err()
        .ok_or_else(|| io::Error::other("expected strict persona validation error"))?;
    assert!(
        error.to_string().contains("metadata.role_class"),
        "unexpected error: {error}"
    );

    Ok(())
}
