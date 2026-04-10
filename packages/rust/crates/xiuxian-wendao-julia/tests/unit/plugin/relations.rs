use std::collections::BTreeMap;

use serde_json::json;
use xiuxian_wendao_core::repo_intelligence::ModuleRecord;

use super::{doc_targets_for_annotation_doc, doc_targets_for_file_doc};

#[test]
fn doc_targets_for_file_doc_links_users_guide_docs_to_owner_modules() {
    let module_lookup = BTreeMap::from([
        (
            "DemoLib".to_string(),
            module("repo:modelica-demo:module:DemoLib", "DemoLib", "package.mo"),
        ),
        (
            "DemoLib.UsersGuide".to_string(),
            module(
                "repo:modelica-demo:module:DemoLib.UsersGuide",
                "DemoLib.UsersGuide",
                "UsersGuide/package.mo",
            ),
        ),
        (
            "DemoLib.Controllers".to_string(),
            module(
                "repo:modelica-demo:module:DemoLib.Controllers",
                "DemoLib.Controllers",
                "Controllers/package.mo",
            ),
        ),
        (
            "DemoLib.Controllers.UsersGuide".to_string(),
            module(
                "repo:modelica-demo:module:DemoLib.Controllers.UsersGuide",
                "DemoLib.Controllers.UsersGuide",
                "Controllers/UsersGuide/package.mo",
            ),
        ),
        (
            "DemoLib.Controllers.UsersGuide.Tutorial".to_string(),
            module(
                "repo:modelica-demo:module:DemoLib.Controllers.UsersGuide.Tutorial",
                "DemoLib.Controllers.UsersGuide.Tutorial",
                "Controllers/UsersGuide/Tutorial/package.mo",
            ),
        ),
    ]);
    let payload = json!([
        {
            "path": "README.md",
            "targets": doc_targets_for_file_doc(
                "README.md",
                "DemoLib",
                &module_lookup,
                Some("repo:modelica-demo:module:DemoLib"),
            ),
        },
        {
            "path": "UsersGuide/Overview.mo",
            "targets": doc_targets_for_file_doc(
                "UsersGuide/Overview.mo",
                "DemoLib",
                &module_lookup,
                Some("repo:modelica-demo:module:DemoLib"),
            ),
        },
        {
            "path": "Controllers/UsersGuide/Tuning.mo",
            "targets": doc_targets_for_file_doc(
                "Controllers/UsersGuide/Tuning.mo",
                "DemoLib",
                &module_lookup,
                Some("repo:modelica-demo:module:DemoLib"),
            ),
        },
        {
            "path": "Controllers/UsersGuide/Tutorial/FirstSteps.mo",
            "targets": doc_targets_for_file_doc(
                "Controllers/UsersGuide/Tutorial/FirstSteps.mo",
                "DemoLib",
                &module_lookup,
                Some("repo:modelica-demo:module:DemoLib"),
            ),
        },
    ]);

    insta::assert_json_snapshot!(
        "doc_targets_for_file_doc_links_users_guide_docs_to_owner_modules",
        payload
    );
}

#[test]
fn doc_targets_for_annotation_doc_links_users_guide_docs_to_owner_modules() {
    let module_lookup = BTreeMap::from([
        (
            "DemoLib".to_string(),
            module("repo:modelica-demo:module:DemoLib", "DemoLib", "package.mo"),
        ),
        (
            "DemoLib.UsersGuide".to_string(),
            module(
                "repo:modelica-demo:module:DemoLib.UsersGuide",
                "DemoLib.UsersGuide",
                "UsersGuide/package.mo",
            ),
        ),
        (
            "DemoLib.Controllers".to_string(),
            module(
                "repo:modelica-demo:module:DemoLib.Controllers",
                "DemoLib.Controllers",
                "Controllers/package.mo",
            ),
        ),
        (
            "DemoLib.Controllers.UsersGuide".to_string(),
            module(
                "repo:modelica-demo:module:DemoLib.Controllers.UsersGuide",
                "DemoLib.Controllers.UsersGuide",
                "Controllers/UsersGuide/package.mo",
            ),
        ),
        (
            "DemoLib.Controllers.UsersGuide.Tutorial".to_string(),
            module(
                "repo:modelica-demo:module:DemoLib.Controllers.UsersGuide.Tutorial",
                "DemoLib.Controllers.UsersGuide.Tutorial",
                "Controllers/UsersGuide/Tutorial/package.mo",
            ),
        ),
    ]);
    let payload = json!([
        {
            "path": "UsersGuide/Overview.mo",
            "targets": doc_targets_for_annotation_doc(
                "UsersGuide/Overview.mo",
                "DemoLib",
                &module_lookup,
                &[],
                Some("repo:modelica-demo:module:DemoLib"),
            ),
        },
        {
            "path": "Controllers/UsersGuide/Tuning.mo",
            "targets": doc_targets_for_annotation_doc(
                "Controllers/UsersGuide/Tuning.mo",
                "DemoLib",
                &module_lookup,
                &[],
                Some("repo:modelica-demo:module:DemoLib"),
            ),
        },
        {
            "path": "Controllers/UsersGuide/Tutorial/FirstSteps.mo",
            "targets": doc_targets_for_annotation_doc(
                "Controllers/UsersGuide/Tutorial/FirstSteps.mo",
                "DemoLib",
                &module_lookup,
                &[],
                Some("repo:modelica-demo:module:DemoLib"),
            ),
        },
    ]);

    insta::assert_json_snapshot!(
        "doc_targets_for_annotation_doc_links_users_guide_docs_to_owner_modules",
        payload
    );
}

fn module(module_id: &str, qualified_name: &str, path: &str) -> ModuleRecord {
    ModuleRecord {
        repo_id: "modelica-demo".to_string(),
        module_id: module_id.to_string(),
        qualified_name: qualified_name.to_string(),
        path: path.to_string(),
    }
}
