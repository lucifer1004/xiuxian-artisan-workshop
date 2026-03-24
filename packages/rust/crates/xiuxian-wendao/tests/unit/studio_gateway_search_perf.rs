use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use anyhow::{Result, anyhow};
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use git2::{IndexAddOption, Repository, Signature, Time};
use serde_json::Value;
use serial_test::serial;
use tower::util::ServiceExt;
use xiuxian_testing::performance::{
    PerfBudget, PerfRunConfig, assert_perf_budget, run_async_budget,
};

use super::repo_index::{RepoCodeDocument, RepoIndexCoordinator};
use super::symbol_index::SymbolIndexCoordinator;
use super::{GatewayState, StudioState, studio_router};
use crate::analyzers::{
    analyze_registered_repository_with_registry, load_repo_intelligence_config,
};
use crate::search_plane::SearchPlaneService;

const PERF_SUITE: &str = "xiuxian-wendao/gateway-search";
const REPO_MODULE_SEARCH_CASE: &str = "repo_module_search";
const REPO_SYMBOL_SEARCH_CASE: &str = "repo_symbol_search";
const REPO_EXAMPLE_SEARCH_CASE: &str = "repo_example_search";
const REPO_PROJECTED_PAGE_SEARCH_CASE: &str = "repo_projected_page_search";
const STUDIO_CODE_SEARCH_CASE: &str = "studio_code_search";
const STUDIO_SEARCH_INDEX_STATUS_CASE: &str = "studio_search_index_status";
const REPO_MODULE_SEARCH_URI: &str =
    "/api/repo/module-search?repo=gateway-sync&query=GatewaySyncPkg&limit=5";
const REPO_SYMBOL_SEARCH_URI: &str =
    "/api/repo/symbol-search?repo=gateway-sync&query=solve&limit=5";
const REPO_EXAMPLE_SEARCH_URI: &str =
    "/api/repo/example-search?repo=gateway-sync&query=solve&limit=5";
const REPO_PROJECTED_PAGE_SEARCH_URI: &str =
    "/api/repo/projected-page-search?repo=gateway-sync&query=solve&limit=5";
const STUDIO_CODE_SEARCH_URI: &str =
    "/api/search/intent?intent=code_search&repo=gateway-sync&q=solve&limit=5";
const STUDIO_SEARCH_INDEX_STATUS_URI: &str = "/api/search/index/status";
const GATEWAY_PERF_ENV_PREFIX: &str = "XIUXIAN_WENDAO_GATEWAY_PERF";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GatewayPerfBudgetProfile {
    Local,
    Linux,
    Macos,
    Windows,
    Other,
}

impl GatewayPerfBudgetProfile {
    fn detect() -> Self {
        std::env::var("RUNNER_OS")
            .ok()
            .as_deref()
            .map_or_else(|| Self::from_label(std::env::consts::OS), Self::from_label)
    }

    fn from_label(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "local" => Self::Local,
            "linux" | "ubuntu" | "ubuntu-latest" => Self::Linux,
            "macos" | "darwin" | "mac" | "macos-latest" => Self::Macos,
            "windows" | "windows-latest" | "win32" => Self::Windows,
            _ => Self::Other,
        }
    }
}

fn perf_run_config() -> PerfRunConfig {
    PerfRunConfig {
        warmup_samples: 1,
        samples: 6,
        timeout_ms: 2_000,
        concurrency: 1,
    }
}

fn perf_budget(case: &str) -> PerfBudget {
    perf_budget_with_lookup(case, GatewayPerfBudgetProfile::detect(), |name| {
        std::env::var(name).ok()
    })
}

fn perf_budget_with_lookup(
    case: &str,
    profile: GatewayPerfBudgetProfile,
    lookup: impl Fn(&str) -> Option<String>,
) -> PerfBudget {
    let default_budget = default_perf_budget(case, profile);
    PerfBudget {
        max_p50_latency_ms: budget_override(&lookup, case, "P50_MS")
            .or(default_budget.max_p50_latency_ms),
        max_p95_latency_ms: budget_override(&lookup, case, "P95_MS")
            .or(default_budget.max_p95_latency_ms),
        max_p99_latency_ms: budget_override(&lookup, case, "P99_MS")
            .or(default_budget.max_p99_latency_ms),
        min_throughput_qps: budget_override(&lookup, case, "MIN_QPS")
            .or(default_budget.min_throughput_qps),
        max_error_rate: budget_override(&lookup, case, "MAX_ERROR_RATE")
            .or(default_budget.max_error_rate),
    }
}

fn budget_override(
    lookup: &impl Fn(&str) -> Option<String>,
    case: &str,
    suffix: &str,
) -> Option<f64> {
    let key = format!(
        "{GATEWAY_PERF_ENV_PREFIX}_{}_{}",
        case.to_ascii_uppercase(),
        suffix
    );
    lookup(&key).and_then(|raw| parse_positive_budget_value(&raw))
}

fn parse_positive_budget_value(raw: &str) -> Option<f64> {
    raw.trim().parse::<f64>().ok().filter(|value| *value > 0.0)
}

fn default_perf_budget(case: &str, profile: GatewayPerfBudgetProfile) -> PerfBudget {
    match profile {
        GatewayPerfBudgetProfile::Linux => linux_perf_budget(case),
        GatewayPerfBudgetProfile::Local
        | GatewayPerfBudgetProfile::Macos
        | GatewayPerfBudgetProfile::Windows
        | GatewayPerfBudgetProfile::Other => default_gateway_perf_budget(case),
    }
}

fn default_gateway_perf_budget(case: &str) -> PerfBudget {
    match case {
        REPO_MODULE_SEARCH_CASE => PerfBudget {
            max_p50_latency_ms: None,
            max_p95_latency_ms: Some(2.0),
            max_p99_latency_ms: None,
            min_throughput_qps: Some(1_101.0),
            max_error_rate: Some(0.001),
        },
        REPO_SYMBOL_SEARCH_CASE => PerfBudget {
            max_p50_latency_ms: None,
            max_p95_latency_ms: Some(2.0),
            max_p99_latency_ms: None,
            min_throughput_qps: Some(964.0),
            max_error_rate: Some(0.001),
        },
        REPO_EXAMPLE_SEARCH_CASE => PerfBudget {
            max_p50_latency_ms: None,
            max_p95_latency_ms: Some(2.5),
            max_p99_latency_ms: None,
            min_throughput_qps: Some(650.0),
            max_error_rate: Some(0.001),
        },
        REPO_PROJECTED_PAGE_SEARCH_CASE => PerfBudget {
            max_p50_latency_ms: None,
            max_p95_latency_ms: Some(2.0),
            max_p99_latency_ms: None,
            min_throughput_qps: Some(650.0),
            max_error_rate: Some(0.001),
        },
        STUDIO_CODE_SEARCH_CASE => PerfBudget {
            max_p50_latency_ms: None,
            max_p95_latency_ms: Some(13.0),
            max_p99_latency_ms: None,
            min_throughput_qps: Some(95.0),
            max_error_rate: Some(0.001),
        },
        STUDIO_SEARCH_INDEX_STATUS_CASE => PerfBudget {
            max_p50_latency_ms: None,
            max_p95_latency_ms: Some(5.0),
            max_p99_latency_ms: None,
            min_throughput_qps: Some(220.0),
            max_error_rate: Some(0.001),
        },
        other => panic!("missing performance budget for gateway case `{other}`"),
    }
}

fn linux_perf_budget(case: &str) -> PerfBudget {
    // Linux CI currently shares the same audited steady-state defaults until a
    // dedicated ubuntu-latest baseline record is captured.
    default_gateway_perf_budget(case)
}

async fn request_status(router: axum::Router, uri: &str) -> Result<StatusCode> {
    let response = router
        .oneshot(Request::builder().uri(uri).body(Body::empty())?)
        .await?;
    let status = response.status();
    let _ = to_bytes(response.into_body(), usize::MAX).await?;
    Ok(status)
}

async fn request_json(router: axum::Router, uri: &str) -> Result<(StatusCode, Value)> {
    let response = router
        .oneshot(Request::builder().uri(uri).body(Body::empty())?)
        .await?;
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await?;
    let payload = serde_json::from_slice(&body)?;
    Ok((status, payload))
}

fn gateway_state_for_project(project_root: &Path) -> Arc<GatewayState> {
    let config_root = project_root.to_path_buf();
    let ui_config = crate::gateway::studio::router::load_ui_config_from_wendao_toml(&config_root)
        .unwrap_or_default();
    let plugin_registry = Arc::new(
        crate::analyzers::bootstrap_builtin_registry()
            .unwrap_or_else(|error| panic!("bootstrap builtin plugin registry: {error}")),
    );
    let repo_index = Arc::new(RepoIndexCoordinator::new(
        project_root.to_path_buf(),
        Arc::clone(&plugin_registry),
        SearchPlaneService::new(project_root.to_path_buf()),
    ));
    repo_index.start();
    let config_path = config_root.join("wendao.toml");
    if config_path.exists() {
        let repo_config = load_repo_intelligence_config(Some(config_path.as_path()), &config_root)
            .unwrap_or_else(|error| {
                panic!("load repo intelligence config for gateway perf tests: {error}")
            });
        for repository in &repo_config.repos {
            analyze_registered_repository_with_registry(
                repository,
                config_root.as_path(),
                &plugin_registry,
            )
            .unwrap_or_else(|error| {
                panic!("prewarm repository analysis cache for gateway perf tests: {error}")
            });
        }
    }

    Arc::new(GatewayState {
        index: None,
        signal_tx: None,
        studio: Arc::new(StudioState {
            project_root: project_root.to_path_buf(),
            config_root,
            ui_config: Arc::new(RwLock::new(ui_config)),
            graph_index: Arc::new(RwLock::new(None)),
            symbol_index: Arc::new(RwLock::new(None)),
            symbol_index_coordinator: Arc::new(SymbolIndexCoordinator::new(
                project_root.to_path_buf(),
                project_root.to_path_buf(),
            )),
            search_plane: SearchPlaneService::new(project_root.to_path_buf()),
            vfs_scan: Arc::new(RwLock::new(None)),
            repo_index,
            plugin_registry,
        }),
    })
}

fn write_default_repo_config(base: &Path, repo_dir: &Path, repo_id: &str) -> Result<()> {
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

fn create_local_git_repo(base: &Path, package_name: &str) -> Result<PathBuf> {
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
        format!(
            "module {package_name}\nexport solve\n\"\"\"solve docs\"\"\"\nsolve() = nothing\nend\n"
        ),
    )?;
    fs::create_dir_all(repo_dir.join("examples"))?;
    fs::write(
        repo_dir.join("examples").join("solve_demo.jl"),
        format!("using {package_name}\nsolve()\n"),
    )?;
    fs::create_dir_all(repo_dir.join("docs"))?;
    fs::write(repo_dir.join("docs").join("solve.md"), "# solve\n")?;

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

fn commit_all(repository: &Repository, message: &str) -> Result<()> {
    let mut index = repository.index()?;
    index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
    index.write()?;
    let tree_id = index.write_tree()?;
    let tree = repository.find_tree(tree_id)?;
    let signature = Signature::new("Gateway Perf", "gateway-perf@example.invalid", &git_time())?;
    let parent = repository
        .head()
        .ok()
        .and_then(|head| head.target())
        .and_then(|target| repository.find_commit(target).ok());

    match parent {
        Some(ref commit) => {
            repository.commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &[commit],
            )?;
        }
        None => {
            repository.commit(Some("HEAD"), &signature, &signature, message, &tree, &[])?;
        }
    }

    Ok(())
}

fn git_time() -> Time {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|error| panic!("system time before unix epoch: {error}"))
        .as_secs();
    let seconds = i64::try_from(seconds).unwrap_or(i64::MAX);
    Time::new(seconds, 0)
}

async fn publish_code_search_snapshot(state: &Arc<GatewayState>, repo_id: &str) -> Result<()> {
    let config_path = state.studio.config_root.join("wendao.toml");
    let config = load_repo_intelligence_config(
        Some(config_path.as_path()),
        state.studio.config_root.as_path(),
    )?;
    let repository = config
        .repos
        .iter()
        .find(|repository| repository.id == repo_id)
        .ok_or_else(|| anyhow!("repository `{repo_id}` not found in perf config"))?;
    let analysis = analyze_registered_repository_with_registry(
        repository,
        state.studio.config_root.as_path(),
        &state.studio.plugin_registry,
    )?;

    state
        .studio
        .search_plane
        .publish_repo_entities_with_revision(repo_id, &analysis, None)
        .await?;
    state
        .studio
        .search_plane
        .publish_repo_content_chunks_with_revision(
            repo_id,
            &[RepoCodeDocument {
                path: "src/GatewaySyncPkg.jl".to_string(),
                language: Some("julia".to_string()),
                contents: Arc::<str>::from(
                    "module GatewaySyncPkg\nexport solve\nsolve() = nothing\nend\n",
                ),
            }],
            None,
        )
        .await?;
    Ok(())
}

async fn prepared_gateway_perf_router()
-> Result<(tempfile::TempDir, Arc<GatewayState>, axum::Router)> {
    let temp = tempfile::tempdir()?;
    let repo_dir = create_local_git_repo(temp.path(), "GatewaySyncPkg")?;
    write_default_repo_config(temp.path(), &repo_dir, "gateway-sync")?;
    let state = gateway_state_for_project(temp.path());
    publish_code_search_snapshot(&state, "gateway-sync").await?;
    Ok((temp, Arc::clone(&state), studio_router(state)))
}

async fn assert_gateway_perf_case(router: axum::Router, case: &str, uri: &'static str) {
    let report = run_async_budget(PERF_SUITE, case, &perf_run_config(), || {
        let router = router.clone();
        async move {
            let status = request_status(router, uri).await?;
            if status == StatusCode::OK {
                Ok::<_, anyhow::Error>(())
            } else {
                Err(anyhow!("unexpected status {status} for {uri}"))
            }
        }
    })
    .await;
    assert_perf_budget(&report, &perf_budget(case));
}

async fn assert_search_index_status_perf_case(router: axum::Router, uri: &'static str) {
    let report = run_async_budget(
        PERF_SUITE,
        STUDIO_SEARCH_INDEX_STATUS_CASE,
        &perf_run_config(),
        || {
            let router = router.clone();
            async move {
                let (status, payload) = request_json(router, uri).await?;
                if status != StatusCode::OK {
                    return Err(anyhow!("unexpected status {status} for {uri}"));
                }
                let summary = payload
                    .get("queryTelemetrySummary")
                    .filter(|value| !value.is_null())
                    .ok_or_else(|| anyhow!("missing queryTelemetrySummary"))?;
                let corpus_count = summary["corpusCount"]
                    .as_u64()
                    .ok_or_else(|| anyhow!("queryTelemetrySummary.corpusCount should be u64"))?;
                let total_rows_scanned = summary["totalRowsScanned"].as_u64().ok_or_else(|| {
                    anyhow!("queryTelemetrySummary.totalRowsScanned should be u64")
                })?;
                if corpus_count == 0 {
                    return Err(anyhow!(
                        "queryTelemetrySummary should report at least one corpus"
                    ));
                }
                if total_rows_scanned == 0 {
                    return Err(anyhow!(
                        "queryTelemetrySummary should report scanned rows after warmup"
                    ));
                }
                Ok::<_, anyhow::Error>(())
            }
        },
    )
    .await;
    assert_perf_budget(&report, &perf_budget(STUDIO_SEARCH_INDEX_STATUS_CASE));
}

async fn assert_prepared_gateway_perf_case(case: &str, uri: &'static str) -> Result<()> {
    let (_temp, _state, router) = prepared_gateway_perf_router().await?;
    assert_gateway_perf_case(router, case, uri).await;
    Ok(())
}

async fn warm_search_index_telemetry_state() -> Result<(tempfile::TempDir, Arc<GatewayState>)> {
    let (temp, state, _router) = prepared_gateway_perf_router().await?;
    let warm_router = studio_router(Arc::clone(&state));
    let warm_status = request_status(warm_router, STUDIO_CODE_SEARCH_URI).await?;
    if warm_status != StatusCode::OK {
        return Err(anyhow!(
            "warmup code-search query returned unexpected status {warm_status}"
        ));
    }
    Ok((temp, state))
}

#[tokio::test]
#[serial(gateway_search_perf)]
#[ignore = "aggregate gateway perf smoke suite; run explicitly when validating the full warm-cache bundle"]
async fn gateway_search_perf_suite_reports_warm_cache_latency() -> Result<()> {
    assert_prepared_gateway_perf_case(REPO_MODULE_SEARCH_CASE, REPO_MODULE_SEARCH_URI).await?;
    assert_prepared_gateway_perf_case(REPO_SYMBOL_SEARCH_CASE, REPO_SYMBOL_SEARCH_URI).await?;
    assert_prepared_gateway_perf_case(REPO_EXAMPLE_SEARCH_CASE, REPO_EXAMPLE_SEARCH_URI).await?;
    assert_prepared_gateway_perf_case(
        REPO_PROJECTED_PAGE_SEARCH_CASE,
        REPO_PROJECTED_PAGE_SEARCH_URI,
    )
    .await?;
    assert_prepared_gateway_perf_case(STUDIO_CODE_SEARCH_CASE, STUDIO_CODE_SEARCH_URI).await?;
    let (_temp, state) = warm_search_index_telemetry_state().await?;
    assert_search_index_status_perf_case(
        studio_router(Arc::clone(&state)),
        STUDIO_SEARCH_INDEX_STATUS_URI,
    )
    .await;
    Ok(())
}

#[test]
fn gateway_perf_budget_profile_maps_runner_labels() {
    assert_eq!(
        GatewayPerfBudgetProfile::from_label("local"),
        GatewayPerfBudgetProfile::Local
    );
    assert_eq!(
        GatewayPerfBudgetProfile::from_label("Linux"),
        GatewayPerfBudgetProfile::Linux
    );
    assert_eq!(
        GatewayPerfBudgetProfile::from_label("macos-latest"),
        GatewayPerfBudgetProfile::Macos
    );
    assert_eq!(
        GatewayPerfBudgetProfile::from_label("windows"),
        GatewayPerfBudgetProfile::Windows
    );
    assert_eq!(
        GatewayPerfBudgetProfile::from_label("custom-runner"),
        GatewayPerfBudgetProfile::Other
    );
}

#[test]
fn gateway_perf_budget_lookup_applies_case_overrides() {
    let budget = perf_budget_with_lookup(
        REPO_MODULE_SEARCH_CASE,
        GatewayPerfBudgetProfile::Local,
        |name| match name {
            "XIUXIAN_WENDAO_GATEWAY_PERF_REPO_MODULE_SEARCH_P95_MS" => Some("3.25".to_string()),
            "XIUXIAN_WENDAO_GATEWAY_PERF_REPO_MODULE_SEARCH_MIN_QPS" => Some("777".to_string()),
            _ => None,
        },
    );

    assert_eq!(budget.max_p95_latency_ms, Some(3.25));
    assert_eq!(budget.min_throughput_qps, Some(777.0));
    assert_eq!(budget.max_error_rate, Some(0.001));
}

#[test]
fn gateway_perf_budget_lookup_ignores_invalid_values() {
    let budget = perf_budget_with_lookup(
        STUDIO_CODE_SEARCH_CASE,
        GatewayPerfBudgetProfile::Local,
        |name| match name {
            "XIUXIAN_WENDAO_GATEWAY_PERF_STUDIO_CODE_SEARCH_P95_MS" => Some("invalid".to_string()),
            "XIUXIAN_WENDAO_GATEWAY_PERF_STUDIO_CODE_SEARCH_MIN_QPS" => Some("-1".to_string()),
            _ => None,
        },
    );

    assert_eq!(budget.max_p95_latency_ms, Some(13.0));
    assert_eq!(budget.min_throughput_qps, Some(95.0));
}
