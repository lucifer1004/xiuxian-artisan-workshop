use super::*;
use crate::gateway::studio::router::StudioState;
use crate::gateway::studio::types::UiConfig;
use serde_json::json;
use tempfile::tempdir;

#[path = "support.rs"]
mod support;
use support::assert_studio_json_snapshot;

struct VfsFixture {
    state: StudioState,
    _temp_dir: tempfile::TempDir,
}

fn make_vfs_fixture() -> VfsFixture {
    let temp_dir =
        tempdir().unwrap_or_else(|err| panic!("failed to create vfs fixture tempdir: {err}"));
    let docs_dir = temp_dir.path().join("docs");
    let skills_dir = temp_dir.path().join("internal_skills").join("writer");
    let knowledge_dir = temp_dir.path().join(".data").join("knowledge");
    let blueprints_dir = temp_dir.path().join(".data").join("blueprints");

    std::fs::create_dir_all(&docs_dir)
        .unwrap_or_else(|err| panic!("failed to create docs dir: {err}"));
    std::fs::create_dir_all(&skills_dir)
        .unwrap_or_else(|err| panic!("failed to create internal skills dir: {err}"));
    std::fs::create_dir_all(&knowledge_dir)
        .unwrap_or_else(|err| panic!("failed to create knowledge dir: {err}"));
    std::fs::create_dir_all(&blueprints_dir)
        .unwrap_or_else(|err| panic!("failed to create blueprints dir: {err}"));

    std::fs::write(docs_dir.join("guide.md"), "# Guide\n\nHello.\n")
        .unwrap_or_else(|err| panic!("failed to write docs fixture: {err}"));
    std::fs::write(skills_dir.join("SKILL.md"), "---\nname: Writer\n---\n")
        .unwrap_or_else(|err| panic!("failed to write skill fixture: {err}"));
    std::fs::write(knowledge_dir.join("context.md"), "# Context\n\nWorld.\n")
        .unwrap_or_else(|err| panic!("failed to write knowledge fixture: {err}"));
    std::fs::write(blueprints_dir.join("default.bpmn"), "<bpmn />")
        .unwrap_or_else(|err| panic!("failed to write blueprint fixture: {err}"));

    let mut state = StudioState::new();
    state.project_root = temp_dir.path().to_path_buf();
    state.data_root = temp_dir.path().join(".data");
    state.knowledge_root = knowledge_dir;
    state.internal_skill_root = temp_dir.path().join("internal_skills");
    state.set_ui_config(UiConfig {
        index_paths: vec!["docs".to_string(), "internal_skills".to_string()],
    });

    VfsFixture {
        state,
        _temp_dir: temp_dir,
    }
}

#[test]
fn scan_roots_includes_configured_and_builtin_roots() {
    let fixture = make_vfs_fixture();

    let result = scan_roots(&fixture.state);
    let mut entries = result
        .entries
        .iter()
        .map(|entry| {
            json!({
                "path": entry.path,
                "name": entry.name,
                "isDir": entry.is_dir,
                "category": entry.category,
                "size": entry.size,
                "contentType": entry.content_type,
                "hasFrontmatter": entry.has_frontmatter,
                "wendaoId": entry.wendao_id,
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));

    assert_studio_json_snapshot(
        "vfs_scan_roots_payload",
        json!({
            "entries": entries,
            "fileCount": result.file_count,
            "dirCount": result.dir_count,
        }),
    );
}

#[test]
fn list_root_entries_reflects_runtime_root_resolution() {
    let fixture = make_vfs_fixture();

    let entries = list_root_entries(&fixture.state);
    let mut roots = entries
        .iter()
        .map(|entry| {
            json!({
                "path": entry.path,
                "name": entry.name,
                "isDir": entry.is_dir,
                "size": entry.size,
                "contentType": entry.content_type,
            })
        })
        .collect::<Vec<_>>();
    roots.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));

    assert_studio_json_snapshot("vfs_root_entries_payload", json!({ "entries": roots }));
}

#[test]
fn get_entry_resolves_configured_relative_roots() {
    let fixture = make_vfs_fixture();

    let entry = get_entry(&fixture.state, "docs/guide.md")
        .unwrap_or_else(|err| panic!("expected docs file entry: {err}"));

    assert_studio_json_snapshot(
        "vfs_get_entry_payload",
        json!({
            "path": entry.path,
            "name": entry.name,
            "isDir": entry.is_dir,
            "size": entry.size,
            "contentType": entry.content_type,
        }),
    );
}

#[tokio::test]
async fn read_content_supports_builtin_blueprints_root() {
    let fixture = make_vfs_fixture();

    let payload = read_content(&fixture.state, "blueprints/default.bpmn")
        .await
        .unwrap_or_else(|err| panic!("expected blueprint content to load: {err}"));

    assert_studio_json_snapshot(
        "vfs_read_content_payload",
        json!({
            "path": payload.path,
            "content": payload.content,
            "contentType": payload.content_type,
        }),
    );
}
