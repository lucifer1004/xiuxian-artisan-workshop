use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::analyzers::{
    RegisteredRepository, RepositoryAnalysisOutput, RepositoryPluginConfig,
    RepositoryRefreshPolicy, analyze_registered_repository_with_registry,
    bootstrap_builtin_registry,
};
use crate::gateway::studio::search::handlers::code_search::search::build_code_search_response;
use crate::gateway::studio::search::handlers::tests::linked_parser_summary::{
    ensure_linked_julia_parser_summary_service, ensure_linked_modelica_parser_summary_service,
};
use crate::gateway::studio::search::handlers::tests::test_studio_state;
use crate::gateway::studio::test_support::{commit_all, init_git_repository};
use crate::repo_index::{
    RepoCodeDocument, RepoIndexEntryStatus, RepoIndexPhase, RepoIndexSnapshot,
};

#[tokio::test]
#[serial_test::serial(julia_live)]
async fn build_code_search_response_returns_hits_for_plain_julia_plugin_repository()
-> Result<(), Box<dyn std::error::Error>> {
    ensure_linked_julia_parser_summary_service()?;
    let temp = tempfile::tempdir()?;
    let repo_dir = create_sample_julia_repo(temp.path(), "SearchJulia")?;
    let repository = RegisteredRepository {
        id: "julia-live".to_string(),
        path: Some(repo_dir.clone()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
    };
    let registry = bootstrap_builtin_registry()?;
    let analysis =
        analyze_registered_repository_with_registry(&repository, temp.path(), &registry)?;

    let studio = test_studio_state();
    studio.set_ui_config(crate::gateway::studio::types::UiConfig {
        projects: Vec::new(),
        repo_projects: vec![crate::gateway::studio::types::UiRepoProjectConfig {
            id: repository.id.clone(),
            root: Some(repo_dir.display().to_string()),
            url: None,
            git_ref: None,
            refresh: None,
            plugins: vec!["julia".to_string()],
        }],
    });
    publish_repository_snapshot(
        &studio,
        &repository.id,
        analysis,
        vec![repo_code_document(
            &repo_dir,
            repo_dir.join("src/SearchJulia.jl"),
            "julia",
        )?],
    )
    .await;

    let response = build_code_search_response(&studio, "solve".to_string(), Some("julia-live"), 10)
        .await
        .unwrap_or_else(|error| panic!("Julia code search response: {error:?}"));

    assert!(
        response.hits.iter().any(
            |hit| hit.doc_type.as_deref() == Some("symbol") && hit.path == "src/SearchJulia.jl"
        ),
        "expected Julia symbol hit in code search response: {:?}",
        response
            .hits
            .iter()
            .map(|hit| (&hit.path, &hit.doc_type))
            .collect::<Vec<_>>()
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial(julia_live)]
async fn build_code_search_response_returns_hits_for_plain_modelica_plugin_repository()
-> Result<(), Box<dyn std::error::Error>> {
    ensure_linked_modelica_parser_summary_service()?;
    let temp = tempfile::tempdir()?;
    let repo_dir = create_sample_modelica_repo(temp.path(), "SearchModelica")?;
    let repository = RegisteredRepository {
        id: "modelica-live".to_string(),
        path: Some(repo_dir.clone()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Id("modelica".to_string())],
    };
    let registry = bootstrap_builtin_registry()?;
    let analysis =
        analyze_registered_repository_with_registry(&repository, temp.path(), &registry)?;

    let studio = test_studio_state();
    studio.set_ui_config(crate::gateway::studio::types::UiConfig {
        projects: Vec::new(),
        repo_projects: vec![crate::gateway::studio::types::UiRepoProjectConfig {
            id: repository.id.clone(),
            root: Some(repo_dir.display().to_string()),
            url: None,
            git_ref: None,
            refresh: None,
            plugins: vec!["modelica".to_string()],
        }],
    });
    publish_repository_snapshot(
        &studio,
        &repository.id,
        analysis,
        vec![
            repo_code_document(&repo_dir, repo_dir.join("package.mo"), "modelica")?,
            repo_code_document(&repo_dir, repo_dir.join("Controllers/PI.mo"), "modelica")?,
        ],
    )
    .await;

    let response = build_code_search_response(&studio, "PI".to_string(), Some("modelica-live"), 10)
        .await
        .unwrap_or_else(|error| panic!("Modelica code search response: {error:?}"));

    assert!(
        response
            .hits
            .iter()
            .any(|hit| hit.doc_type.as_deref() == Some("symbol") && hit.path == "Controllers/PI.mo"),
        "expected Modelica symbol hit in code search response: {:?}",
        response
            .hits
            .iter()
            .map(|hit| (&hit.path, &hit.doc_type))
            .collect::<Vec<_>>()
    );
    Ok(())
}

async fn publish_repository_snapshot(
    studio: &crate::gateway::studio::router::StudioState,
    repo_id: &str,
    analysis: RepositoryAnalysisOutput,
    documents: Vec<RepoCodeDocument>,
) {
    let analysis = Arc::new(analysis);
    studio
        .search_plane
        .publish_repo_entities_with_revision(repo_id, analysis.as_ref(), &documents, None)
        .await
        .unwrap_or_else(|error| panic!("publish repo entities for `{repo_id}`: {error}"));
    studio
        .search_plane
        .publish_repo_content_chunks_with_revision(repo_id, &documents, None)
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks for `{repo_id}`: {error}"));
    studio
        .repo_index
        .set_snapshot_for_test(&Arc::new(RepoIndexSnapshot {
            repo_id: repo_id.to_string(),
            analysis: Arc::clone(&analysis),
        }));
    studio.repo_index.set_status_for_test(RepoIndexEntryStatus {
        repo_id: repo_id.to_string(),
        phase: RepoIndexPhase::Ready,
        queue_position: None,
        last_error: None,
        last_revision: Some("fixture".to_string()),
        updated_at: Some("2026-04-09T00:00:00Z".to_string()),
        attempt_count: 1,
    });
}

fn repo_code_document(
    repo_root: &Path,
    file_path: impl AsRef<Path>,
    language: &str,
) -> Result<RepoCodeDocument, Box<dyn std::error::Error>> {
    let file_path = file_path.as_ref();
    let contents = fs::read_to_string(file_path)?;
    let relative_path = file_path
        .strip_prefix(repo_root)?
        .to_string_lossy()
        .replace('\\', "/");
    Ok(RepoCodeDocument {
        path: relative_path,
        language: Some(language.to_string()),
        size_bytes: u64::try_from(contents.len()).unwrap_or(u64::MAX),
        modified_unix_ms: 0,
        contents: Arc::<str>::from(contents),
    })
}

fn create_sample_julia_repo(
    base: &Path,
    package_name: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let repo_dir = base.join(package_name.to_ascii_lowercase());
    fs::create_dir_all(repo_dir.join("src"))?;
    fs::write(
        repo_dir.join("Project.toml"),
        format!(
            "name = \"{package_name}\"\nuuid = \"12345678-1234-1234-1234-123456789abc\"\nversion = \"0.1.0\"\n"
        ),
    )?;
    fs::write(
        repo_dir.join("src").join(format!("{package_name}.jl")),
        format!(
            r#"module {package_name}

export solve, Problem

struct Problem
    x::Int
end

function solve(problem::Problem)
    problem.x
end
end
"#
        ),
    )?;
    initialize_git_fixture(&repo_dir, package_name);
    Ok(repo_dir)
}

fn create_sample_modelica_repo(
    base: &Path,
    package_name: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let repo_dir = base.join(package_name.to_ascii_lowercase());
    fs::create_dir_all(repo_dir.join("Controllers"))?;
    fs::write(repo_dir.join("package.order"), "Controllers\n")?;
    fs::write(
        repo_dir.join("package.mo"),
        format!("within;\npackage {package_name}\nend {package_name};\n"),
    )?;
    fs::write(
        repo_dir.join("Controllers").join("package.mo"),
        format!("within {package_name};\npackage Controllers\nend Controllers;\n"),
    )?;
    fs::write(
        repo_dir.join("Controllers").join("PI.mo"),
        format!("within {package_name}.Controllers;\nmodel PI\n  parameter Real k = 1;\nend PI;\n"),
    )?;
    initialize_git_fixture(&repo_dir, package_name);
    Ok(repo_dir)
}

fn initialize_git_fixture(repo_dir: &Path, package_name: &str) {
    init_git_repository(repo_dir);
    let remote_url = format!(
        "https://example.invalid/xiuxian-wendao/{}.git",
        package_name.to_ascii_lowercase()
    );
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(["remote", "add", "origin", remote_url.as_str()])
        .output()
        .unwrap_or_else(|error| panic!("add git remote: {error}"));
    assert!(
        output.status.success(),
        "add git remote failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    commit_all(repo_dir, "initial import");
}
