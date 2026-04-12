use std::path::PathBuf;

use xiuxian_wendao_runtime::artifacts::zhixing::{
    ZHIXING_SKILL_DOC_PATH, embedded_resource_text, embedded_resource_text_from_wendao_uri,
    embedded_semantic_reference_mounts, embedded_skill_markdown,
};

fn some_or_panic<T>(value: Option<T>, context: &str) -> T {
    match value {
        Some(value) => value,
        None => panic!("{context}"),
    }
}

#[test]
fn embedded_skill_markdown_loads_agenda_management_manifest() {
    let markdown = some_or_panic(
        embedded_skill_markdown(),
        "agenda-management skill markdown",
    );
    assert!(markdown.contains("name: agenda-management"));
    assert!(markdown.contains("Skill Manifest: Agenda Management"));
}

#[test]
fn embedded_resource_text_normalizes_relative_paths() {
    let normalized = some_or_panic(
        embedded_resource_text("zhixing/templates/daily_agenda.md"),
        "daily agenda template",
    );
    let relative = some_or_panic(
        embedded_resource_text("./zhixing/templates/daily_agenda.md"),
        "relative daily agenda template",
    );

    assert_eq!(normalized, relative);
    assert!(normalized.contains("# Daily Agenda"));
}

#[test]
fn embedded_resource_text_from_wendao_uri_reads_skill_reference() {
    let uri = "wendao://skills/agenda-management/references/steward.md#persona";
    let content = some_or_panic(
        embedded_resource_text_from_wendao_uri(uri),
        "steward reference",
    );

    assert!(content.contains("Professional Identity: The Clockwork Guardian"));
}

#[test]
fn embedded_semantic_mounts_include_agenda_management_references() {
    let mounts = embedded_semantic_reference_mounts();
    let Some(paths) = mounts.get("agenda-management") else {
        panic!("agenda-management mount");
    };

    assert_eq!(
        paths,
        &vec![PathBuf::from("zhixing/skills/agenda-management/references")]
    );
}

#[test]
fn embedded_skill_doc_path_matches_runtime_resource_contract() {
    assert_eq!(
        ZHIXING_SKILL_DOC_PATH,
        "zhixing/skills/agenda-management/SKILL.md"
    );
}
