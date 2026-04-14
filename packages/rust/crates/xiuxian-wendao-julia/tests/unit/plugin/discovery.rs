use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde_json::json;
use tempfile::TempDir;

use super::{
    RepositorySnapshot, RepositorySurface, doc_format_hint, doc_sort_key, doc_title,
    documented_nested_users_guide_topics, documented_release_notes_topics, example_sort_key,
    is_supported_users_guide_doc_path, module_sort_key, repository_surface,
    synthetic_section_title,
};

fn surface_name(surface: RepositorySurface) -> &'static str {
    match surface {
        RepositorySurface::Api => "api",
        RepositorySurface::Example => "example",
        RepositorySurface::Documentation => "documentation",
        RepositorySurface::Support => "support",
    }
}

#[test]
fn repository_snapshot_preloads_modelica_entries_and_package_orders() {
    let tempdir = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    fs::write(
        tempdir.path().join("package.mo"),
        "within ;\npackage DemoLib\nend DemoLib;\n",
    )
    .unwrap_or_else(|error| panic!("write root package: {error}"));
    fs::create_dir_all(tempdir.path().join("Blocks"))
        .unwrap_or_else(|error| panic!("create Blocks dir: {error}"));
    fs::write(
        tempdir.path().join("Blocks/package.mo"),
        "within DemoLib;\npackage Blocks\nend Blocks;\n",
    )
    .unwrap_or_else(|error| panic!("write nested package: {error}"));
    fs::write(
        tempdir.path().join("Blocks/package.order"),
        "Interfaces\nUtilities\n",
    )
    .unwrap_or_else(|error| panic!("write package.order: {error}"));
    fs::write(tempdir.path().join("README.md"), "# Demo\n")
        .unwrap_or_else(|error| panic!("write readme: {error}"));

    let snapshot = RepositorySnapshot::load(tempdir.path())
        .unwrap_or_else(|error| panic!("load snapshot: {error}"));
    let payload = json!({
        "entries": snapshot
            .entries()
            .iter()
            .map(|entry| json!({
                "relative_path": entry.relative_path,
                "surface": surface_name(entry.surface),
                "has_modelica_contents": entry.modelica_contents.is_some(),
            }))
            .collect::<Vec<_>>(),
        "package_orders": snapshot.package_orders(),
        "package_files": snapshot
            .package_files()
            .unwrap_or_else(|error| panic!("package files: {error}"))
            .into_iter()
            .map(|entry| entry.relative_path.clone())
            .collect::<Vec<_>>(),
    });

    insta::assert_json_snapshot!(
        "repository_snapshot_preloads_modelica_entries_and_package_orders",
        payload
    );
}

#[test]
fn module_sort_key_uses_package_order_hierarchy() {
    let orders = BTreeMap::from([
        (
            String::new(),
            vec!["UsersGuide".to_string(), "Controllers".to_string()],
        ),
        (
            "Controllers".to_string(),
            vec!["Examples".to_string(), "PI".to_string()],
        ),
    ]);
    let payload = json!([
        {
            "path": "package.mo",
            "key": module_sort_key("package.mo", &orders),
        },
        {
            "path": "UsersGuide/package.mo",
            "key": module_sort_key("UsersGuide/package.mo", &orders),
        },
        {
            "path": "Controllers/package.mo",
            "key": module_sort_key("Controllers/package.mo", &orders),
        },
        {
            "path": "Controllers/Examples/package.mo",
            "key": module_sort_key("Controllers/Examples/package.mo", &orders),
        },
    ]);

    insta::assert_json_snapshot!("module_sort_key_uses_package_order_hierarchy", payload);
}

#[test]
fn example_sort_key_uses_package_order_leaf_entries() {
    let orders = BTreeMap::from([
        (String::new(), vec!["Controllers".to_string()]),
        ("Controllers".to_string(), vec!["Examples".to_string()]),
        (
            "Controllers/Examples".to_string(),
            vec!["Step".to_string(), "Alpha".to_string()],
        ),
    ]);
    let payload = json!([
        {
            "path": "Controllers/Examples/Step.mo",
            "key": example_sort_key("Controllers/Examples/Step.mo", &orders),
        },
        {
            "path": "Controllers/Examples/Alpha.mo",
            "key": example_sort_key("Controllers/Examples/Alpha.mo", &orders),
        },
    ]);

    insta::assert_json_snapshot!("example_sort_key_uses_package_order_leaf_entries", payload);
}

#[test]
fn detects_repository_surfaces() {
    let payload = json!([
        {
            "path": "Controllers/Examples/Step.mo",
            "surface": surface_name(repository_surface("Controllers/Examples/Step.mo")),
        },
        {
            "path": "Controllers/Examples/ExampleUtilities/Helper.mo",
            "surface": surface_name(repository_surface(
                "Controllers/Examples/ExampleUtilities/Helper.mo",
            )),
        },
        {
            "path": "Controllers/Examples/Utilities/Helper.mo",
            "surface": surface_name(repository_surface("Controllers/Examples/Utilities/Helper.mo")),
        },
        {
            "path": "Controllers/Internal/Helper.mo",
            "surface": surface_name(repository_surface("Controllers/Internal/Helper.mo")),
        },
        {
            "path": "Controllers/PI.mo",
            "surface": surface_name(repository_surface("Controllers/PI.mo")),
        },
        {
            "path": "UsersGuide/Overview.mo",
            "surface": surface_name(repository_surface("UsersGuide/Overview.mo")),
        },
    ]);

    insta::assert_json_snapshot!("detects_repository_surfaces", payload);
}

#[test]
#[allow(clippy::too_many_lines)]
fn infers_users_guide_doc_formats() {
    let payload = json!([
        {
            "path": "Controllers/UsersGuide/package.mo",
            "file_format": doc_format_hint("Controllers/UsersGuide/package.mo", false),
            "annotation_format": doc_format_hint("Controllers/UsersGuide/package.mo", true),
        },
        {
            "path": "Controllers/UsersGuide/Conventions.mo",
            "file_format": doc_format_hint("Controllers/UsersGuide/Conventions.mo", false),
            "annotation_format": doc_format_hint("Controllers/UsersGuide/Conventions.mo", true),
        },
        {
            "path": "Controllers/UsersGuide/Connectors.mo",
            "file_format": doc_format_hint("Controllers/UsersGuide/Connectors.mo", false),
            "annotation_format": doc_format_hint("Controllers/UsersGuide/Connectors.mo", true),
        },
        {
            "path": "Controllers/UsersGuide/Implementation.mo",
            "file_format": doc_format_hint("Controllers/UsersGuide/Implementation.mo", false),
            "annotation_format": doc_format_hint("Controllers/UsersGuide/Implementation.mo", true),
        },
        {
            "path": "Controllers/UsersGuide/RevisionHistory.mo",
            "file_format": doc_format_hint("Controllers/UsersGuide/RevisionHistory.mo", false),
            "annotation_format": doc_format_hint("Controllers/UsersGuide/RevisionHistory.mo", true),
        },
        {
            "path": "Controllers/UsersGuide/VersionManagement.mo",
            "file_format": doc_format_hint("Controllers/UsersGuide/VersionManagement.mo", false),
            "annotation_format": doc_format_hint("Controllers/UsersGuide/VersionManagement.mo", true),
        },
        {
            "path": "Controllers/UsersGuide/Tutorial/package.mo",
            "file_format": doc_format_hint("Controllers/UsersGuide/Tutorial/package.mo", false),
            "annotation_format": doc_format_hint("Controllers/UsersGuide/Tutorial/package.mo", true),
        },
        {
            "path": "Controllers/UsersGuide/Tutorial/FirstSteps.mo",
            "file_format": doc_format_hint("Controllers/UsersGuide/Tutorial/FirstSteps.mo", false),
            "annotation_format": doc_format_hint("Controllers/UsersGuide/Tutorial/FirstSteps.mo", true),
        },
        {
            "path": "Controllers/UsersGuide/ReleaseNotes.mo",
            "file_format": doc_format_hint("Controllers/UsersGuide/ReleaseNotes.mo", false),
            "annotation_format": doc_format_hint("Controllers/UsersGuide/ReleaseNotes.mo", true),
        },
        {
            "path": "Controllers/UsersGuide/References.mo",
            "file_format": doc_format_hint("Controllers/UsersGuide/References.mo", false),
            "annotation_format": doc_format_hint("Controllers/UsersGuide/References.mo", true),
        },
        {
            "path": "Controllers/UsersGuide/Contact.mo",
            "file_format": doc_format_hint("Controllers/UsersGuide/Contact.mo", false),
            "annotation_format": doc_format_hint("Controllers/UsersGuide/Contact.mo", true),
        },
        {
            "path": "Controllers/UsersGuide/Concept.mo",
            "file_format": doc_format_hint("Controllers/UsersGuide/Concept.mo", false),
            "annotation_format": doc_format_hint("Controllers/UsersGuide/Concept.mo", true),
        },
        {
            "path": "Controllers/UsersGuide/Parameters.mo",
            "file_format": doc_format_hint("Controllers/UsersGuide/Parameters.mo", false),
            "annotation_format": doc_format_hint("Controllers/UsersGuide/Parameters.mo", true),
        },
        {
            "path": "UsersGuide/Overview.mo",
            "file_format": doc_format_hint("UsersGuide/Overview.mo", false),
            "annotation_format": doc_format_hint("UsersGuide/Overview.mo", true),
        },
        {
            "path": "UsersGuide/Conventions.mo",
            "file_format": doc_format_hint("UsersGuide/Conventions.mo", false),
            "annotation_format": doc_format_hint("UsersGuide/Conventions.mo", true),
        },
        {
            "path": "UsersGuide/Connectors.mo",
            "file_format": doc_format_hint("UsersGuide/Connectors.mo", false),
            "annotation_format": doc_format_hint("UsersGuide/Connectors.mo", true),
        },
        {
            "path": "UsersGuide/Implementation.mo",
            "file_format": doc_format_hint("UsersGuide/Implementation.mo", false),
            "annotation_format": doc_format_hint("UsersGuide/Implementation.mo", true),
        },
        {
            "path": "UsersGuide/RevisionHistory.mo",
            "file_format": doc_format_hint("UsersGuide/RevisionHistory.mo", false),
            "annotation_format": doc_format_hint("UsersGuide/RevisionHistory.mo", true),
        },
        {
            "path": "UsersGuide/VersionManagement.mo",
            "file_format": doc_format_hint("UsersGuide/VersionManagement.mo", false),
            "annotation_format": doc_format_hint("UsersGuide/VersionManagement.mo", true),
        },
        {
            "path": "UsersGuide/Literature.mo",
            "file_format": doc_format_hint("UsersGuide/Literature.mo", false),
            "annotation_format": doc_format_hint("UsersGuide/Literature.mo", true),
        },
        {
            "path": "UsersGuide/Glossar.mo",
            "file_format": doc_format_hint("UsersGuide/Glossar.mo", false),
            "annotation_format": doc_format_hint("UsersGuide/Glossar.mo", true),
        },
        {
            "path": "UsersGuide/Parameterization.mo",
            "file_format": doc_format_hint("UsersGuide/Parameterization.mo", false),
            "annotation_format": doc_format_hint("UsersGuide/Parameterization.mo", true),
        },
    ]);

    insta::assert_json_snapshot!("infers_users_guide_doc_formats", payload);
}

#[test]
fn detects_nested_users_guide_topics_from_conventions_files() {
    let payload = json!({
        "conventions": documented_nested_users_guide_topics(
            "package Conventions\n  package Documentation\n    annotation (Documentation(info=\"<html>Doc.</html>\"));\n  end Documentation;\n  package ModelicaCode\n    annotation (Documentation(info=\"<html>Code.</html>\"));\n  end ModelicaCode;\n  class Icons\n    annotation (Documentation(info=\"<html>Icons.</html>\"));\n  end Icons;\nend Conventions;\n"
        )
        .into_iter()
        .map(|topic| json!({
            "title": topic.title,
            "format": topic.format,
        }))
        .collect::<Vec<_>>(),
        "non_conventions": documented_nested_users_guide_topics(
            "model Overview\n  annotation (Documentation(info=\"<html>Overview.</html>\"));\nend Overview;\n"
        )
        .into_iter()
        .map(|topic| json!({
            "title": topic.title,
            "format": topic.format,
        }))
        .collect::<Vec<_>>(),
    });

    insta::assert_json_snapshot!(
        "detects_nested_users_guide_topics_from_conventions_files",
        payload
    );
}

#[test]
fn detects_release_notes_topics_from_nested_release_notes_files() {
    let payload = json!({
        "release_notes": documented_release_notes_topics(
            "package ReleaseNotes\n  class VersionManagement\n    annotation (Documentation(info=\"<html>Version workflow.</html>\"));\n  end VersionManagement;\n  class Version_4_1_0\n    annotation (Documentation(info=\"<html>Release 4.1.0.</html>\"));\n  end Version_4_1_0;\n  class Version_4_0_0\n    annotation (Documentation(info=\"<html>Release 4.0.0.</html>\"));\n  end Version_4_0_0;\nend ReleaseNotes;\n"
        )
        .into_iter()
        .map(|topic| json!({
            "title": topic.title,
            "format": topic.format,
        }))
        .collect::<Vec<_>>(),
        "generic_page": documented_release_notes_topics(
            "model Overview\n  annotation (Documentation(info=\"<html>Overview.</html>\"));\nend Overview;\n"
        )
        .into_iter()
        .map(|topic| json!({
            "title": topic.title,
            "format": topic.format,
        }))
        .collect::<Vec<_>>(),
    });

    insta::assert_json_snapshot!(
        "detects_release_notes_topics_from_nested_release_notes_files",
        payload
    );
}

#[test]
fn normalizes_synthetic_section_titles() {
    let payload = json!([
        {
            "raw": "Documentation",
            "title": synthetic_section_title("Documentation"),
        },
        {
            "raw": "ModelicaCode",
            "title": synthetic_section_title("ModelicaCode"),
        },
        {
            "raw": "VersionManagement",
            "title": synthetic_section_title("VersionManagement"),
        },
        {
            "raw": "Version_4_1_0",
            "title": synthetic_section_title("Version_4_1_0"),
        },
    ]);

    insta::assert_json_snapshot!("normalizes_synthetic_section_titles", payload);
}

#[test]
fn doc_sort_key_uses_package_order_and_annotation_position() {
    let orders = BTreeMap::from([
        (String::new(), vec!["Controllers".to_string()]),
        ("Controllers".to_string(), vec!["UsersGuide".to_string()]),
        (
            "Controllers/UsersGuide".to_string(),
            vec![
                "Tutorial".to_string(),
                "References".to_string(),
                "ReleaseNotes".to_string(),
                "Tuning".to_string(),
            ],
        ),
        (
            "Controllers/UsersGuide/Tutorial".to_string(),
            vec!["FirstSteps".to_string()],
        ),
    ]);
    let payload = json!([
        {
            "path": "Controllers/UsersGuide/package.mo",
            "key": doc_sort_key("Controllers/UsersGuide/package.mo", &orders),
        },
        {
            "path": "Controllers/UsersGuide/Tutorial/package.mo",
            "key": doc_sort_key("Controllers/UsersGuide/Tutorial/package.mo", &orders),
        },
        {
            "path": "Controllers/UsersGuide/Tutorial/FirstSteps.mo",
            "key": doc_sort_key("Controllers/UsersGuide/Tutorial/FirstSteps.mo", &orders),
        },
        {
            "path": "Controllers/UsersGuide/Tutorial/FirstSteps.mo#annotation.documentation",
            "key": doc_sort_key(
                "Controllers/UsersGuide/Tutorial/FirstSteps.mo#annotation.documentation",
                &orders,
            ),
        },
        {
            "path": "Controllers/UsersGuide/Conventions.mo#section.Documentation",
            "key": doc_sort_key(
                "Controllers/UsersGuide/Conventions.mo#section.Documentation",
                &orders,
            ),
        },
        {
            "path": "Controllers/UsersGuide/References.mo",
            "key": doc_sort_key("Controllers/UsersGuide/References.mo", &orders),
        },
        {
            "path": "Controllers/UsersGuide/ReleaseNotes.mo#section.VersionManagement",
            "key": doc_sort_key(
                "Controllers/UsersGuide/ReleaseNotes.mo#section.VersionManagement",
                &orders,
            ),
        },
        {
            "path": "Controllers/UsersGuide/ReleaseNotes.mo",
            "key": doc_sort_key("Controllers/UsersGuide/ReleaseNotes.mo", &orders),
        },
        {
            "path": "Controllers/UsersGuide/Tuning.mo",
            "key": doc_sort_key("Controllers/UsersGuide/Tuning.mo", &orders),
        },
    ]);

    insta::assert_json_snapshot!(
        "doc_sort_key_uses_package_order_and_annotation_position",
        payload
    );
}

#[test]
fn filters_supported_users_guide_doc_assets() {
    let payload = json!([
        {
            "path": "UsersGuide/package.mo",
            "supported": is_supported_users_guide_doc_path(Path::new("UsersGuide/package.mo")),
        },
        {
            "path": "UsersGuide/Overview.mo",
            "supported": is_supported_users_guide_doc_path(Path::new("UsersGuide/Overview.mo")),
        },
        {
            "path": "UsersGuide/Guide.md",
            "supported": is_supported_users_guide_doc_path(Path::new("UsersGuide/Guide.md")),
        },
        {
            "path": "UsersGuide/package.order",
            "supported": is_supported_users_guide_doc_path(Path::new("UsersGuide/package.order")),
        },
    ]);

    insta::assert_json_snapshot!("filters_supported_users_guide_doc_assets", payload);
}

#[test]
fn normalizes_doc_titles_from_paths() {
    let payload = json!([
        {
            "path": "README.md",
            "title": doc_title(Path::new("README.md")),
        },
        {
            "path": "UsersGuide/package.mo",
            "title": doc_title(Path::new("UsersGuide/package.mo")),
        },
        {
            "path": "UsersGuide/Overview.mo",
            "title": doc_title(Path::new("UsersGuide/Overview.mo")),
        },
        {
            "path": "UsersGuide/Guide.md",
            "title": doc_title(Path::new("UsersGuide/Guide.md")),
        },
    ]);

    insta::assert_json_snapshot!("normalizes_doc_titles_from_paths", payload);
}
