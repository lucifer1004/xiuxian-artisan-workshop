#![cfg(feature = "zhenfa-router")]

use crate as xiuxian_wendao;

use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use git2::{IndexAddOption, Repository, Signature, Time};
use serde_json::Value;
use tower::util::ServiceExt;

use xiuxian_wendao::gateway::studio::test_support::assert_studio_json_snapshot;
use xiuxian_wendao::gateway::studio::{GatewayState, StudioState, studio_router};
use xiuxian_wendao::repo_intelligence::{
    ProjectedPageIndexNode, ProjectionPageKind, RepoProjectedPageIndexTreesQuery,
    RepoProjectedPagesQuery, repo_projected_page_index_trees_from_config,
    repo_projected_pages_from_config,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

async fn request_json(
    router: axum::Router,
    uri: &str,
) -> Result<(StatusCode, Value), Box<dyn std::error::Error>> {
    let response = router
        .oneshot(Request::builder().uri(uri).body(Body::empty())?)
        .await?;
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await?;
    let payload = serde_json::from_slice(&body)?;
    Ok((status, payload))
}

#[tokio::test]
async fn repo_overview_endpoint_returns_repo_summary_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(router, "/api/repo/overview?repo=gateway-sync").await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_overview_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_module_search_endpoint_returns_module_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/module-search?repo=gateway-sync&query=GatewaySyncPkg&limit=5",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_module_search_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_symbol_search_endpoint_returns_symbol_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\nsolve() = nothing\nend\n",
    )?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/symbol-search?repo=gateway-sync&query=solve&limit=5",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_symbol_search_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_example_search_endpoint_returns_example_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/example-search?repo=gateway-sync&query=solve&limit=5",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_example_search_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_doc_coverage_endpoint_returns_coverage_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    fs::write(repo_dir.join("docs").join("Problem.md"), "# Problem\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/doc-coverage?repo=gateway-sync&module=GatewaySyncPkg",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_doc_coverage_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_sync_endpoint_returns_repo_status_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, mut payload) =
        request_json(router, "/api/repo/sync?repo=gateway-sync&mode=status").await?;
    assert_eq!(status, StatusCode::OK);
    redact_repo_sync_payload(&mut payload);
    assert_studio_json_snapshot("repo_sync_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_pages_endpoint_returns_projection_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) =
        request_json(router, "/api/repo/projected-pages?repo=gateway-sync").await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_projected_pages_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_endpoint_returns_projection_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-page?repo=gateway-sync&page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_projected_page_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_index_tree_endpoint_returns_tree_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-page-index-tree?repo=gateway-sync&page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_projected_page_index_tree_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_index_node_endpoint_returns_node_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-page-index-node?repo=gateway-sync&page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md&node_id=reference/solve-69592caeddee%23anchors",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_projected_page_index_node_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_index_tree_search_endpoint_returns_hit_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-page-index-tree-search?repo=gateway-sync&query=anchors&kind=reference&limit=5",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot(
        "repo_projected_page_index_tree_search_endpoint_json",
        payload,
    );
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_search_endpoint_returns_projection_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-page-search?repo=gateway-sync&query=solve&kind=reference&limit=5",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_projected_page_search_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_retrieval_endpoint_returns_mixed_hit_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-retrieval?repo=gateway-sync&query=solve&kind=reference&limit=5",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_projected_retrieval_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_retrieval_hit_endpoint_returns_page_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-retrieval-hit?repo=gateway-sync&page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_projected_retrieval_hit_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_retrieval_context_endpoint_returns_node_context_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-retrieval-context?repo=gateway-sync&page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md&node_id=reference/solve-69592caeddee%23anchors&related_limit=3",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_projected_retrieval_context_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_family_context_endpoint_returns_family_clusters() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let pages = repo_projected_pages_from_config(
        &RepoProjectedPagesQuery {
            repo_id: "gateway-sync".to_string(),
        },
        None,
        temp.path(),
    )?;
    let page = pages
        .pages
        .iter()
        .find(|page| page.kind == ProjectionPageKind::HowTo)
        .expect("expected a projected how-to page");
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        &format!(
            "/api/repo/projected-page-family-context?repo=gateway-sync&page_id={}&per_kind_limit=2",
            page.page_id
        ),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_projected_page_family_context_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_family_search_endpoint_returns_family_clusters() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-page-family-search?repo=gateway-sync&query=solve&kind=reference&limit=5&per_kind_limit=2",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_projected_page_family_search_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_family_cluster_endpoint_returns_family_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let pages = repo_projected_pages_from_config(
        &RepoProjectedPagesQuery {
            repo_id: "gateway-sync".to_string(),
        },
        None,
        temp.path(),
    )?;
    let page = pages
        .pages
        .iter()
        .find(|page| {
            page.kind == ProjectionPageKind::Reference
                && page.title == "GatewaySyncPkg.solve"
                && page.page_id.contains(":symbol:")
        })
        .expect("expected a symbol-backed projected reference page titled `GatewaySyncPkg.solve`");
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        &format!(
            "/api/repo/projected-page-family-cluster?repo=gateway-sync&page_id={}&kind=how_to&limit=2",
            page.page_id
        ),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_projected_page_family_cluster_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_navigation_endpoint_returns_navigation_bundle() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let pages = repo_projected_pages_from_config(
        &RepoProjectedPagesQuery {
            repo_id: "gateway-sync".to_string(),
        },
        None,
        temp.path(),
    )?;
    let page = pages
        .pages
        .iter()
        .find(|page| {
            page.kind == ProjectionPageKind::Reference
                && page.title == "GatewaySyncPkg.solve"
                && page.page_id.contains(":symbol:")
        })
        .expect("expected a symbol-backed projected reference page titled `GatewaySyncPkg.solve`");
    let trees = repo_projected_page_index_trees_from_config(
        &RepoProjectedPageIndexTreesQuery {
            repo_id: "gateway-sync".to_string(),
        },
        None,
        temp.path(),
    )?;
    let tree = trees
        .trees
        .iter()
        .find(|tree| tree.page_id == page.page_id)
        .expect("expected a projected page-index tree for the selected page");
    let node_id = find_node_id(tree.roots.as_slice(), "Anchors")
        .expect("expected a projected page-index node titled `Anchors`");
    let encoded_node_id = node_id.replace('#', "%23");
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        &format!(
            "/api/repo/projected-page-navigation?repo=gateway-sync&page_id={}&node_id={}&family_kind=how_to&related_limit=3&family_limit=2",
            page.page_id, encoded_node_id
        ),
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_projected_page_navigation_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_navigation_search_endpoint_returns_navigation_hits() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-page-navigation-search?repo=gateway-sync&query=solve&kind=reference&family_kind=how_to&limit=5&related_limit=3&family_limit=2",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot(
        "repo_projected_page_navigation_search_endpoint_json",
        payload,
    );
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_index_trees_endpoint_returns_tree_payload() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-page-index-trees?repo=gateway-sync",
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_studio_json_snapshot("repo_projected_page_index_trees_endpoint_json", payload);
    Ok(())
}

#[tokio::test]
async fn repo_gateway_returns_missing_repo_error() -> TestResult {
    let temp = tempfile::tempdir()?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    for uri in [
        "/api/repo/overview",
        "/api/repo/module-search?query=solve",
        "/api/repo/symbol-search?query=solve",
        "/api/repo/example-search?query=solve",
        "/api/repo/projected-page?page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md",
        "/api/repo/projected-page-index-node?page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md&node_id=reference/solve-69592caeddee%23anchors",
        "/api/repo/projected-retrieval-hit?page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md",
        "/api/repo/projected-retrieval-context?page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md",
        "/api/repo/projected-page-family-context?page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md",
        "/api/repo/projected-page-family-search?query=solve",
        "/api/repo/projected-page-family-cluster?page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md&kind=reference",
        "/api/repo/projected-page-navigation?page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md",
        "/api/repo/projected-page-navigation-search?query=solve",
        "/api/repo/projected-page-index-tree?page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md",
        "/api/repo/projected-page-index-tree-search?query=anchors",
        "/api/repo/projected-page-search?query=solve",
        "/api/repo/projected-retrieval?query=solve",
        "/api/repo/doc-coverage",
        "/api/repo/sync",
        "/api/repo/projected-pages",
        "/api/repo/projected-page-index-trees",
    ] {
        let (status, payload) = request_json(router.clone(), uri).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
        assert_studio_json_snapshot("repo_gateway_missing_repo_error", payload);
    }
    Ok(())
}

#[tokio::test]
async fn repo_gateway_search_endpoints_require_query_param() -> TestResult {
    let temp = tempfile::tempdir()?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    for uri in [
        "/api/repo/module-search?repo=gateway-sync",
        "/api/repo/symbol-search?repo=gateway-sync",
        "/api/repo/example-search?repo=gateway-sync",
        "/api/repo/projected-page-index-tree-search?repo=gateway-sync",
        "/api/repo/projected-page-search?repo=gateway-sync",
        "/api/repo/projected-page-family-search?repo=gateway-sync",
        "/api/repo/projected-page-navigation-search?repo=gateway-sync",
        "/api/repo/projected-retrieval?repo=gateway-sync",
    ] {
        let (status, payload) = request_json(router.clone(), uri).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
        assert_studio_json_snapshot("repo_gateway_missing_query_error", payload);
    }
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_endpoint_requires_page_id() -> TestResult {
    let temp = tempfile::tempdir()?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    for uri in [
        "/api/repo/projected-page?repo=gateway-sync",
        "/api/repo/projected-page-index-node?repo=gateway-sync&node_id=reference/solve-69592caeddee%23anchors",
        "/api/repo/projected-retrieval-hit?repo=gateway-sync",
        "/api/repo/projected-retrieval-context?repo=gateway-sync",
        "/api/repo/projected-page-family-context?repo=gateway-sync",
        "/api/repo/projected-page-family-cluster?repo=gateway-sync&kind=reference",
        "/api/repo/projected-page-navigation?repo=gateway-sync",
        "/api/repo/projected-page-index-tree?repo=gateway-sync",
    ] {
        let (status, payload) = request_json(router.clone(), uri).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
        assert_studio_json_snapshot("repo_gateway_missing_page_id_error", payload);
    }
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_index_node_endpoint_requires_node_id() -> TestResult {
    let temp = tempfile::tempdir()?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-page-index-node?repo=gateway-sync&page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md",
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_studio_json_snapshot("repo_gateway_missing_node_id_error", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_family_cluster_endpoint_requires_kind() -> TestResult {
    let temp = tempfile::tempdir()?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-page-family-cluster?repo=gateway-sync&page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md",
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_studio_json_snapshot("repo_gateway_missing_kind_error", payload);
    Ok(())
}

#[tokio::test]
async fn repo_sync_endpoint_rejects_invalid_mode() -> TestResult {
    let temp = tempfile::tempdir()?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) =
        request_json(router, "/api/repo/sync?repo=gateway-sync&mode=bogus").await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_studio_json_snapshot("repo_sync_endpoint_invalid_mode_error", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_search_endpoint_rejects_invalid_kind() -> TestResult {
    let temp = tempfile::tempdir()?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    for uri in [
        "/api/repo/projected-page-search?repo=gateway-sync&query=solve&kind=bogus",
        "/api/repo/projected-page-family-cluster?repo=gateway-sync&page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md&kind=bogus",
        "/api/repo/projected-page-family-search?repo=gateway-sync&query=solve&kind=bogus",
        "/api/repo/projected-page-navigation?repo=gateway-sync&page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md&family_kind=bogus",
        "/api/repo/projected-page-navigation-search?repo=gateway-sync&query=solve&family_kind=bogus",
        "/api/repo/projected-page-navigation-search?repo=gateway-sync&query=solve&kind=bogus",
        "/api/repo/projected-page-index-tree-search?repo=gateway-sync&query=anchors&kind=bogus",
        "/api/repo/projected-retrieval?repo=gateway-sync&query=solve&kind=bogus",
    ] {
        let (status, payload) = request_json(router.clone(), uri).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}");
        assert_studio_json_snapshot("repo_projected_page_search_invalid_kind_error", payload);
    }
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_endpoint_returns_not_found_for_unknown_page() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-page?repo=gateway-sync&page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/missing.md",
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_studio_json_snapshot("repo_projected_page_not_found_error", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_index_tree_endpoint_returns_not_found_for_unknown_page() -> TestResult
{
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-page-index-tree?repo=gateway-sync&page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/missing.md",
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_studio_json_snapshot("repo_projected_page_index_tree_not_found_error", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_index_node_endpoint_returns_not_found_for_unknown_node() -> TestResult
{
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-page-index-node?repo=gateway-sync&page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md&node_id=reference/solve-69592caeddee%23missing",
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_studio_json_snapshot("repo_projected_page_index_node_not_found_error", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_retrieval_hit_endpoint_returns_not_found_for_unknown_node() -> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-retrieval-hit?repo=gateway-sync&page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md&node_id=reference/solve-69592caeddee%23missing",
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_studio_json_snapshot("repo_projected_retrieval_hit_not_found_error", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_retrieval_context_endpoint_returns_not_found_for_unknown_node() -> TestResult
{
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-retrieval-context?repo=gateway-sync&page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md&node_id=reference/solve-69592caeddee%23missing",
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_studio_json_snapshot("repo_projected_retrieval_context_not_found_error", payload);
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_family_context_endpoint_returns_not_found_for_unknown_page()
-> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        "/api/repo/projected-page-family-context?repo=gateway-sync&page_id=repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/missing.md",
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_studio_json_snapshot(
        "repo_projected_page_family_context_not_found_error",
        payload,
    );
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_family_cluster_endpoint_returns_not_found_for_unknown_family()
-> TestResult {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let pages = repo_projected_pages_from_config(
        &RepoProjectedPagesQuery {
            repo_id: "gateway-sync".to_string(),
        },
        None,
        temp.path(),
    )?;
    let page = pages
        .pages
        .iter()
        .find(|page| {
            page.kind == ProjectionPageKind::Reference
                && page.title == "GatewaySyncPkg.solve"
                && page.page_id.contains(":symbol:")
        })
        .expect("expected a symbol-backed projected reference page titled `GatewaySyncPkg.solve`");
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        &format!(
            "/api/repo/projected-page-family-cluster?repo=gateway-sync&page_id={}&kind=tutorial&limit=2",
            page.page_id
        ),
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_studio_json_snapshot(
        "repo_projected_page_family_cluster_not_found_error",
        payload,
    );
    Ok(())
}

#[tokio::test]
async fn repo_projected_page_navigation_endpoint_returns_not_found_for_unknown_family() -> TestResult
{
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    fs::write(
        repo_dir.join("src").join("GatewaySyncPkg.jl"),
        "module GatewaySyncPkg\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n",
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        "using GatewaySyncPkg\nsolve()\n",
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let pages = repo_projected_pages_from_config(
        &RepoProjectedPagesQuery {
            repo_id: "gateway-sync".to_string(),
        },
        None,
        temp.path(),
    )?;
    let page = pages
        .pages
        .iter()
        .find(|page| {
            page.kind == ProjectionPageKind::Reference
                && page.title == "GatewaySyncPkg.solve"
                && page.page_id.contains(":symbol:")
        })
        .expect("expected a symbol-backed projected reference page titled `GatewaySyncPkg.solve`");
    let router = studio_router(gateway_state_for_project(temp.path()));

    let (status, payload) = request_json(
        router,
        &format!(
            "/api/repo/projected-page-navigation?repo=gateway-sync&page_id={}&family_kind=tutorial&family_limit=2",
            page.page_id
        ),
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_studio_json_snapshot("repo_projected_page_navigation_not_found_error", payload);
    Ok(())
}

fn gateway_state_for_project(project_root: &Path) -> Arc<GatewayState> {
    Arc::new(GatewayState {
        index: None,
        signal_tx: None,
        studio: Arc::new(StudioState {
            project_root: project_root.to_path_buf(),
            config_root: project_root.join(".config"),
            ui_config: Arc::new(RwLock::new(
                xiuxian_wendao::gateway::studio::types::UiConfig {
                    projects: Vec::new(),
                    repo_projects: Vec::new(),
                },
            )),
            graph_index: Arc::new(RwLock::new(None)),
            symbol_index: Arc::new(RwLock::new(None)),
            ast_index: Arc::new(RwLock::new(None)),
        }),
    })
}

fn write_default_repo_config(
    base: &Path,
    repo_dir: &Path,
    repo_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(
        base.join("wendao.toml"),
        format!(
            r#"[link_graph.projects.{repo_id}]
root = "{}"
plugins = ["julia"]
"#,
            repo_dir.display()
        ),
    )?;
    Ok(())
}

fn create_local_git_repo(
    base: &Path,
    package_name: &str,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let repo_dir = base.join(package_name.to_ascii_lowercase());
    fs::create_dir_all(repo_dir.join("src"))?;
    fs::write(repo_dir.join("README.md"), "# Gateway Repo\n")?;
    fs::write(
        repo_dir.join("Project.toml"),
        format!(
            r#"name = "{package_name}"
uuid = "12345678-1234-1234-1234-123456789abc"
version = "0.1.0"
"#
        ),
    )?;
    fs::write(
        repo_dir.join("src").join(format!("{package_name}.jl")),
        format!("module {package_name}\nend\n"),
    )?;

    let repository = Repository::init(&repo_dir)?;
    repository.remote(
        "origin",
        &format!(
            "https://example.invalid/xiuxian-wendao/{}.git",
            package_name.to_ascii_lowercase()
        ),
    )?;
    commit_all(&repository, "initial import")?;
    Ok(repo_dir)
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

fn redact_repo_sync_payload(value: &mut Value) {
    if let Some(path) = value.pointer_mut("/checkout_path") {
        *path = Value::String("[checkout-path]".to_string());
    }
    if let Some(path) = value.pointer_mut("/mirror_path") {
        *path = Value::String("[mirror-path]".to_string());
    }
    if let Some(url) = value.pointer_mut("/upstream_url") {
        *url = Value::String("[upstream-url]".to_string());
    }
    if let Some(path) = value.pointer_mut("/checked_at") {
        *path = Value::String("[checked-at]".to_string());
    }
    if let Some(path) = value.pointer_mut("/last_fetched_at") {
        *path = match path {
            Value::Null => Value::Null,
            _ => Value::String("[last-fetched-at]".to_string()),
        };
    }
    if let Some(path) = value.pointer_mut("/status_summary/freshness/checked_at") {
        *path = Value::String("[checked-at]".to_string());
    }
    if let Some(path) = value.pointer_mut("/status_summary/freshness/last_fetched_at") {
        *path = match path {
            Value::Null => Value::Null,
            _ => Value::String("[last-fetched-at]".to_string()),
        };
    }
}

fn find_node_id(nodes: &[ProjectedPageIndexNode], title: &str) -> Option<String> {
    for node in nodes {
        if node.title == title {
            return Some(node.node_id.clone());
        }
        if let Some(node_id) = find_node_id(node.children.as_slice(), title) {
            return Some(node_id);
        }
    }
    None
}
