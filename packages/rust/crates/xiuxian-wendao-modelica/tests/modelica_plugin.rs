//! Integration tests for the external Modelica Repo Intelligence plugin.

use std::fs;
use std::path::{Path, PathBuf};

use git2::{IndexAddOption, Repository, Signature, Time};
use serde_json::json;
use xiuxian_wendao::repo_intelligence::{
    DocCoverageQuery, ExampleSearchQuery, ModuleSearchQuery, RepoOverviewQuery, SymbolSearchQuery,
    analyze_repository_from_config_with_registry, bootstrap_builtin_registry,
    build_projected_page_index_documents, build_projected_page_index_trees, build_projected_pages,
    build_projection_inputs, doc_coverage_from_config_with_registry,
    example_search_from_config_with_registry, module_search_from_config_with_registry,
    repo_overview_from_config_with_registry, symbol_search_from_config_with_registry,
};
use xiuxian_wendao_modelica::register_into;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn modelica_plugin_supports_registry_aware_repo_queries() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_modelica_repo(temp.path())?;
    let config_path = write_repo_config(temp.path(), &repo_dir)?;

    let mut registry = bootstrap_builtin_registry()?;
    register_into(&mut registry)?;
    let analysis = analyze_repository_from_config_with_registry(
        "modelica-demo",
        Some(&config_path),
        temp.path(),
        &registry,
    )?;

    let payload = json!({
        "overview": repo_overview_from_config_with_registry(
            &RepoOverviewQuery {
                repo_id: "modelica-demo".to_string(),
            },
            Some(&config_path),
            temp.path(),
            &registry,
        )?,
        "module_search": module_search_from_config_with_registry(
            &ModuleSearchQuery {
                repo_id: "modelica-demo".to_string(),
                query: "DemoLib".to_string(),
                limit: 10,
            },
            Some(&config_path),
            temp.path(),
            &registry,
        )?,
        "support_module_search": module_search_from_config_with_registry(
            &ModuleSearchQuery {
                repo_id: "modelica-demo".to_string(),
                query: "Internal".to_string(),
                limit: 10,
            },
            Some(&config_path),
            temp.path(),
            &registry,
        )?,
        "symbol_search": symbol_search_from_config_with_registry(
            &SymbolSearchQuery {
                repo_id: "modelica-demo".to_string(),
                query: "PI".to_string(),
                limit: 10,
            },
            Some(&config_path),
            temp.path(),
            &registry,
        )?,
        "example_search": example_search_from_config_with_registry(
            &ExampleSearchQuery {
                repo_id: "modelica-demo".to_string(),
                query: "Controllers".to_string(),
                limit: 10,
            },
            Some(&config_path),
            temp.path(),
            &registry,
        )?,
        "doc_coverage": doc_coverage_from_config_with_registry(
            &DocCoverageQuery {
                repo_id: "modelica-demo".to_string(),
                module_id: Some("DemoLib.Controllers".to_string()),
            },
            Some(&config_path),
            temp.path(),
            &registry,
        )?,
        "users_guide_doc_coverage": doc_coverage_from_config_with_registry(
            &DocCoverageQuery {
                repo_id: "modelica-demo".to_string(),
                module_id: Some("DemoLib.Controllers.UsersGuide".to_string()),
            },
            Some(&config_path),
            temp.path(),
            &registry,
        )?,
        "root_users_guide_doc_coverage": doc_coverage_from_config_with_registry(
            &DocCoverageQuery {
                repo_id: "modelica-demo".to_string(),
                module_id: Some("DemoLib.UsersGuide".to_string()),
            },
            Some(&config_path),
            temp.path(),
            &registry,
        )?,
        "projection_inputs": build_projection_inputs(&analysis),
        "projected_pages": build_projected_pages(&analysis),
        "projected_page_index_documents": build_projected_page_index_documents(&analysis)?
            .into_iter()
            .filter(|document| {
                matches!(
                    document.title.as_str(),
                    "First Steps" | "Version 4.1.0" | "PI documentation"
                )
            })
            .collect::<Vec<_>>(),
        "projected_page_index_trees": build_projected_page_index_trees(&analysis)?
            .into_iter()
            .filter(|tree| {
                matches!(
                    tree.title.as_str(),
                    "First Steps" | "Version 4.1.0" | "PI documentation"
                )
            })
            .collect::<Vec<_>>(),
        "plugin_ids": registry.plugin_ids(),
    });

    insta::assert_json_snapshot!("modelica_plugin_queries", payload);
    Ok(())
}

fn write_repo_config(base: &Path, repo_dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let config_path = base.join("wendao.toml");
    fs::write(
        &config_path,
        format!(
            r#"[repo_intelligence]
enabled = true

[[repo_intelligence.repos]]
id = "modelica-demo"
path = "{}"
plugins = ["modelica"]
"#,
            repo_dir.display()
        ),
    )?;
    Ok(config_path)
}

fn create_modelica_repo(base: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let repo_dir = base.join("demolib");
    fs::create_dir_all(repo_dir.join("Controllers").join("Examples"))?;
    fs::create_dir_all(
        repo_dir
            .join("Controllers")
            .join("Examples")
            .join("ExampleUtilities"),
    )?;
    fs::create_dir_all(
        repo_dir
            .join("Controllers")
            .join("Examples")
            .join("Utilities"),
    )?;
    fs::create_dir_all(repo_dir.join("Controllers").join("Internal"))?;
    fs::create_dir_all(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("Tutorial"),
    )?;
    fs::create_dir_all(repo_dir.join("Controllers").join("UsersGuide"))?;
    fs::create_dir_all(repo_dir.join("UsersGuide"))?;
    fs::write(repo_dir.join("README.md"), "# DemoLib\n")?;
    fs::write(repo_dir.join("package.order"), "UsersGuide\nControllers\n")?;
    fs::write(
        repo_dir.join("UsersGuide").join("package.order"),
        "Overview\nConventions\nConnectors\nImplementation\nRevisionHistory\nVersionManagement\nReleaseNotes\nParameterization\nGlossar\nContact\n",
    )?;
    fs::write(
        repo_dir.join("package.mo"),
        "within;\npackage DemoLib\n  annotation(Documentation(info = \"<html>DemoLib package docs.</html>\"));\nend DemoLib;\n",
    )?;
    fs::write(
        repo_dir.join("Controllers").join("package.mo"),
        "within DemoLib;\npackage Controllers\nend Controllers;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("package.order"),
        "Tutorial\nConventions\nConnectors\nImplementation\nRevisionHistory\nVersionManagement\nConcept\nReferences\nReleaseNotes\nTuning\nParameters\nContact\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("package.mo"),
        "within DemoLib.Controllers;\npackage UsersGuide\nend UsersGuide;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("Examples")
            .join("package.order"),
        "Step\nAlpha\n",
    )?;
    fs::write(
        repo_dir.join("Controllers").join("PI.mo"),
        "within DemoLib.Controllers;\nmodel PI\n  annotation(Documentation(info = \"<html>PI controller docs.</html>\"));\nend PI;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("Conventions.mo"),
        "within DemoLib.Controllers.UsersGuide;\npackage Conventions\n  annotation(Documentation(info = \"<html>Controller conventions.</html>\"));\n  package Documentation\n    annotation(Documentation(info = \"<html>Controller documentation conventions.</html>\"));\n  end Documentation;\n  package ModelicaCode\n    annotation(Documentation(info = \"<html>Controller Modelica code conventions.</html>\"));\n  end ModelicaCode;\n  class Icons\n    annotation(Documentation(info = \"<html>Controller icon conventions.</html>\"));\n  end Icons;\nend Conventions;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("Connectors.mo"),
        "within DemoLib.Controllers.UsersGuide;\nmodel Connectors\n  annotation(Documentation(info = \"<html>Controller connector guide.</html>\"));\nend Connectors;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("Implementation.mo"),
        "within DemoLib.Controllers.UsersGuide;\nmodel Implementation\n  annotation(Documentation(info = \"<html>Controller implementation notes.</html>\"));\nend Implementation;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("RevisionHistory.mo"),
        "within DemoLib.Controllers.UsersGuide;\nmodel RevisionHistory\n  annotation(Documentation(info = \"<html>Controller revision history.</html>\"));\nend RevisionHistory;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("VersionManagement.mo"),
        "within DemoLib.Controllers.UsersGuide;\nmodel VersionManagement\n  annotation(Documentation(info = \"<html>Controller version management.</html>\"));\nend VersionManagement;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("Concept.mo"),
        "within DemoLib.Controllers.UsersGuide;\nmodel Concept\n  annotation(Documentation(info = \"<html>Controller concept guide.</html>\"));\nend Concept;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("Tuning.mo"),
        "within DemoLib.Controllers.UsersGuide;\nmodel Tuning\n  annotation(Documentation(info = \"<html>Tuning guide.</html>\"));\nend Tuning;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("Parameters.mo"),
        "within DemoLib.Controllers.UsersGuide;\nmodel Parameters\n  annotation(Documentation(info = \"<html>Controller parameters.</html>\"));\nend Parameters;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("ReleaseNotes.mo"),
        "within DemoLib.Controllers.UsersGuide;\npackage ReleaseNotes\n  annotation(Documentation(info = \"<html>Controller release notes.</html>\"));\n  class VersionManagement\n    annotation(Documentation(info = \"<html>Controller release workflow.</html>\"));\n  end VersionManagement;\n  class Version_4_1_0\n    annotation(Documentation(info = \"<html>Controller release 4.1.0.</html>\"));\n  end Version_4_1_0;\nend ReleaseNotes;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("References.mo"),
        "within DemoLib.Controllers.UsersGuide;\nmodel References\n  annotation(Documentation(info = \"<html>Controller references.</html>\"));\nend References;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("Contact.mo"),
        "within DemoLib.Controllers.UsersGuide;\nmodel Contact\n  annotation(Documentation(info = \"<html>Controller contact.</html>\"));\nend Contact;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("Tutorial")
            .join("package.order"),
        "FirstSteps\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("Tutorial")
            .join("package.mo"),
        "within DemoLib.Controllers.UsersGuide;\npackage Tutorial\nend Tutorial;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("UsersGuide")
            .join("Tutorial")
            .join("FirstSteps.mo"),
        "within DemoLib.Controllers.UsersGuide.Tutorial;\nmodel FirstSteps\n  annotation(Documentation(info = \"<html>First steps guide.</html>\"));\nend FirstSteps;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("Examples")
            .join("Step.mo"),
        "within DemoLib.Controllers.Examples;\nmodel Step\nend Step;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("Examples")
            .join("Alpha.mo"),
        "within DemoLib.Controllers.Examples;\nmodel Alpha\nend Alpha;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("Examples")
            .join("ExampleUtilities")
            .join("package.mo"),
        "within DemoLib.Controllers.Examples;\npackage ExampleUtilities\nend ExampleUtilities;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("Examples")
            .join("ExampleUtilities")
            .join("Helper.mo"),
        "within DemoLib.Controllers.Examples.ExampleUtilities;\nmodel Helper\nend Helper;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("Examples")
            .join("Utilities")
            .join("package.mo"),
        "within DemoLib.Controllers.Examples;\npackage Utilities\nend Utilities;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("Examples")
            .join("Utilities")
            .join("Support.mo"),
        "within DemoLib.Controllers.Examples.Utilities;\nmodel Support\nend Support;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("Internal")
            .join("package.mo"),
        "within DemoLib.Controllers;\npackage Internal\nend Internal;\n",
    )?;
    fs::write(
        repo_dir
            .join("Controllers")
            .join("Internal")
            .join("Helper.mo"),
        "within DemoLib.Controllers.Internal;\nmodel Helper\nend Helper;\n",
    )?;
    fs::write(
        repo_dir.join("UsersGuide").join("package.mo"),
        "within DemoLib;\npackage UsersGuide\nend UsersGuide;\n",
    )?;
    fs::write(
        repo_dir.join("UsersGuide").join("Overview.mo"),
        "within DemoLib.UsersGuide;\nmodel Overview\n  annotation(Documentation(info = \"<html>Overview guide.</html>\"));\nend Overview;\n",
    )?;
    fs::write(
        repo_dir.join("UsersGuide").join("Conventions.mo"),
        "within DemoLib.UsersGuide;\npackage Conventions\n  annotation(Documentation(info = \"<html>Root conventions guide.</html>\"));\n  package Documentation\n    annotation(Documentation(info = \"<html>Root documentation conventions.</html>\"));\n  end Documentation;\n  package ModelicaCode\n    annotation(Documentation(info = \"<html>Root Modelica code conventions.</html>\"));\n  end ModelicaCode;\n  class Icons\n    annotation(Documentation(info = \"<html>Root icon conventions.</html>\"));\n  end Icons;\nend Conventions;\n",
    )?;
    fs::write(
        repo_dir.join("UsersGuide").join("Connectors.mo"),
        "within DemoLib.UsersGuide;\nmodel Connectors\n  annotation(Documentation(info = \"<html>Root connector guide.</html>\"));\nend Connectors;\n",
    )?;
    fs::write(
        repo_dir.join("UsersGuide").join("Implementation.mo"),
        "within DemoLib.UsersGuide;\nmodel Implementation\n  annotation(Documentation(info = \"<html>Root implementation notes.</html>\"));\nend Implementation;\n",
    )?;
    fs::write(
        repo_dir.join("UsersGuide").join("RevisionHistory.mo"),
        "within DemoLib.UsersGuide;\nmodel RevisionHistory\n  annotation(Documentation(info = \"<html>Root revision history.</html>\"));\nend RevisionHistory;\n",
    )?;
    fs::write(
        repo_dir.join("UsersGuide").join("VersionManagement.mo"),
        "within DemoLib.UsersGuide;\nmodel VersionManagement\n  annotation(Documentation(info = \"<html>Root version management.</html>\"));\nend VersionManagement;\n",
    )?;
    fs::write(
        repo_dir.join("UsersGuide").join("ReleaseNotes.mo"),
        "within DemoLib.UsersGuide;\npackage ReleaseNotes\n  annotation(Documentation(info = \"<html>Root release notes.</html>\"));\n  class VersionManagement\n    annotation(Documentation(info = \"<html>Root release workflow.</html>\"));\n  end VersionManagement;\n  class Version_4_0_0\n    annotation(Documentation(info = \"<html>Root release 4.0.0.</html>\"));\n  end Version_4_0_0;\nend ReleaseNotes;\n",
    )?;
    fs::write(
        repo_dir.join("UsersGuide").join("Glossar.mo"),
        "within DemoLib.UsersGuide;\nmodel Glossar\n  annotation(Documentation(info = \"<html>Glossary entries.</html>\"));\nend Glossar;\n",
    )?;
    fs::write(
        repo_dir.join("UsersGuide").join("Parameterization.mo"),
        "within DemoLib.UsersGuide;\nmodel Parameterization\n  annotation(Documentation(info = \"<html>Parameterization notes.</html>\"));\nend Parameterization;\n",
    )?;
    fs::write(
        repo_dir.join("UsersGuide").join("Contact.mo"),
        "within DemoLib.UsersGuide;\nmodel Contact\n  annotation(Documentation(info = \"<html>Root contact page.</html>\"));\nend Contact;\n",
    )?;

    initialize_git_repository(&repo_dir)?;
    Ok(repo_dir)
}

fn initialize_git_repository(repo_dir: &Path) -> TestResult {
    let repository = Repository::init(repo_dir)?;
    repository.remote(
        "origin",
        "https://example.invalid/xiuxian-wendao/demolib.git",
    )?;
    commit_all(&repository, "initial import")?;
    Ok(())
}

fn commit_all(repository: &Repository, message: &str) -> Result<(), git2::Error> {
    let mut index = repository.index()?;
    index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repository.find_tree(tree_id)?;
    let signature = Signature::new(
        "Xiuxian Test",
        "test@example.com",
        &Time::new(1_700_000_000, 0),
    )?;
    repository.commit(Some("HEAD"), &signature, &signature, message, &tree, &[])?;
    Ok(())
}
