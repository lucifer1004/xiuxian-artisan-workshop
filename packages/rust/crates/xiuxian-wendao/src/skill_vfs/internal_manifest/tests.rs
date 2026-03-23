use std::fs;
use std::path::Path;

use tempfile::{Builder, tempdir};
use xiuxian_skills::InternalSkillWorkflowType as SkillWorkflowType;

use super::{
    InternalSkillWorkflowType, load_internal_skill_manifest_from_path,
    resolve_internal_skill_authority,
};

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directories");
    }
    fs::write(path, content).expect("write file");
}

#[test]
fn load_manifest_uses_defaults_and_overrides() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("sample.toml");
    write_file(
        &path,
        r#"
name = "Sample Tool"
description = "Sample description"
internal_id = "sample-native"
mcp_contract = { category = "filesystem" }
workflow_type = { type = "workflow" }
qianhuan_background = { background = "wendao://background" }
flow_definition = { uri = "flow://definition" }
annotations = { read_only = true, destructive = false, idempotent = true, open_world = false }
"#,
    );

    let manifest = load_internal_skill_manifest_from_path(&path).expect("manifest");

    assert_eq!(manifest.manifest_id, "sample");
    assert_eq!(manifest.tool_name, "Sample Tool");
    assert_eq!(manifest.description, "Sample description");
    assert_eq!(manifest.internal_id, "sample-native");
    assert_eq!(manifest.workflow_type, SkillWorkflowType::QianjiFlow);
    assert_eq!(
        manifest.qianhuan_background.as_deref(),
        Some("wendao://background")
    );
    assert_eq!(
        manifest.flow_definition.as_deref(),
        Some("flow://definition")
    );
    assert_eq!(
        manifest.metadata,
        serde_json::json!({ "category": "filesystem" })
    );
    assert!(manifest.annotations.read_only);
    assert!(!manifest.annotations.destructive);
    assert!(manifest.annotations.is_idempotent());
    assert!(!manifest.annotations.is_open_world());
    assert_eq!(manifest.source_path, path);
}

#[test]
fn load_manifest_rejects_invalid_description() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("sample.toml");
    write_file(
        &path,
        r#"
description = "invalid"
"#,
    );

    let error = load_internal_skill_manifest_from_path(&path).expect_err("expected failure");
    assert!(error.to_string().contains("invalid description"));
}

#[test]
fn workflow_type_parser_recognizes_known_aliases() {
    assert_eq!(
        InternalSkillWorkflowType::from_raw(None),
        InternalSkillWorkflowType::Qianji
    );
    assert_eq!(
        InternalSkillWorkflowType::from_raw(Some("flow")),
        InternalSkillWorkflowType::Qianji
    );
    assert!(matches!(
        InternalSkillWorkflowType::from_raw(Some("native")),
        InternalSkillWorkflowType::Unknown(value) if value == "native"
    ));
}

#[test]
fn resolve_authority_collects_authorized_ghost_and_unauthorized_manifests() {
    let dir = Builder::new()
        .prefix("internal-manifest")
        .tempdir_in(".")
        .expect("tempdir");
    let root = dir.path();
    let root_rel = Path::new(root.file_name().expect("tempdir name"));

    let alpha_root = root.join("alpha");
    write_file(
        &alpha_root.join("SKILL.md"),
        r#"
[manifest](references/qianji.toml)
[ghost](references/missing/qianji.toml)
"#,
    );
    write_file(
        &alpha_root.join("references/qianji.toml"),
        r#"
manifest_id = "alpha-manifest"
name = "Alpha Tool"
"#,
    );

    let beta_root = root.join("beta");
    write_file(
        &beta_root.join("SKILL.md"),
        r#"
beta skill without explicit manifest links
"#,
    );
    write_file(
        &beta_root.join("references/qianji.toml"),
        r#"
manifest_id = "beta-manifest"
name = "Beta Tool"
"#,
    );

    let outcome = resolve_internal_skill_authority(root_rel).expect("authority outcome");

    assert_eq!(
        outcome.report.authorized_manifests,
        vec!["wendao://skills-internal/alpha/references/qianji.toml"]
    );
    assert_eq!(
        outcome.report.ghost_links,
        vec!["wendao://skills-internal/alpha/references/missing/qianji.toml"]
    );
    assert_eq!(
        outcome.report.unauthorized_manifests,
        vec!["wendao://skills-internal/beta/references/qianji.toml"]
    );
    assert_eq!(outcome.authorized.len(), 1);
    assert_eq!(outcome.authorized[0].tool_name, "Alpha Tool");
    assert_eq!(
        outcome.authorized[0].workflow_type,
        SkillWorkflowType::QianjiFlow
    );
    assert!(
        outcome.authorized[0]
            .source_path
            .ends_with("alpha/references/qianji.toml")
    );
}
