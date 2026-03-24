use anyhow::{Result, anyhow};
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use serde_json::Value;
use serial_test::file_serial;
use tower::util::ServiceExt;
use xiuxian_testing::{PerfBudget, PerfRunConfig, assert_perf_budget, run_async_budget};
use xiuxian_wendao::gateway::studio::perf_support::{
    GatewayPerfFixture, prepare_gateway_perf_fixture,
};

const SUITE: &str = "xiuxian-wendao/perf-gateway";
const REPO_MODULE_SEARCH_CASE: &str = "repo_module_search_formal";
const REPO_SYMBOL_SEARCH_CASE: &str = "repo_symbol_search_formal";
const REPO_EXAMPLE_SEARCH_CASE: &str = "repo_example_search_formal";
const REPO_PROJECTED_PAGE_SEARCH_CASE: &str = "repo_projected_page_search_formal";
const STUDIO_CODE_SEARCH_CASE: &str = "studio_code_search_formal";
const STUDIO_SEARCH_INDEX_STATUS_CASE: &str = "studio_search_index_status_formal";
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

fn perf_run_config() -> PerfRunConfig {
    PerfRunConfig {
        warmup_samples: 1,
        samples: 6,
        timeout_ms: 2_000,
        concurrency: 1,
    }
}

fn perf_budget(case: &str) -> PerfBudget {
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
        other => panic!("missing performance budget for formal gateway case `{other}`"),
    }
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

async fn assert_status_perf_case(
    fixture: &GatewayPerfFixture,
    case: &str,
    uri: &'static str,
) -> Result<()> {
    let report = run_async_budget(SUITE, case, &perf_run_config(), || {
        let router = fixture.router();
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
    Ok(())
}

#[tokio::test]
#[file_serial(formal_gateway_search_perf)]
async fn repo_module_search_perf_gate_reports_warm_cache_latency_formal_gate() -> Result<()> {
    let fixture = prepare_gateway_perf_fixture().await?;
    assert_status_perf_case(&fixture, REPO_MODULE_SEARCH_CASE, REPO_MODULE_SEARCH_URI).await
}

#[tokio::test]
#[file_serial(formal_gateway_search_perf)]
async fn repo_symbol_search_perf_gate_reports_warm_cache_latency_formal_gate() -> Result<()> {
    let fixture = prepare_gateway_perf_fixture().await?;
    assert_status_perf_case(&fixture, REPO_SYMBOL_SEARCH_CASE, REPO_SYMBOL_SEARCH_URI).await
}

#[tokio::test]
#[file_serial(formal_gateway_search_perf)]
async fn repo_example_search_perf_gate_reports_warm_cache_latency_formal_gate() -> Result<()> {
    let fixture = prepare_gateway_perf_fixture().await?;
    assert_status_perf_case(&fixture, REPO_EXAMPLE_SEARCH_CASE, REPO_EXAMPLE_SEARCH_URI).await
}

#[tokio::test]
#[file_serial(formal_gateway_search_perf)]
async fn repo_projected_page_search_perf_gate_reports_warm_cache_latency_formal_gate() -> Result<()>
{
    let fixture = prepare_gateway_perf_fixture().await?;
    assert_status_perf_case(
        &fixture,
        REPO_PROJECTED_PAGE_SEARCH_CASE,
        REPO_PROJECTED_PAGE_SEARCH_URI,
    )
    .await
}

#[tokio::test]
#[file_serial(formal_gateway_search_perf)]
async fn studio_code_search_perf_gate_reports_warm_cache_latency_formal_gate() -> Result<()> {
    let fixture = prepare_gateway_perf_fixture().await?;
    assert_status_perf_case(&fixture, STUDIO_CODE_SEARCH_CASE, STUDIO_CODE_SEARCH_URI).await
}

#[tokio::test]
#[file_serial(formal_gateway_search_perf)]
async fn search_index_status_perf_gate_reports_query_telemetry_summary_formal_gate() -> Result<()> {
    let fixture = prepare_gateway_perf_fixture().await?;
    let warm_status = request_status(fixture.router(), STUDIO_CODE_SEARCH_URI).await?;
    if warm_status != StatusCode::OK {
        return Err(anyhow!(
            "warmup code-search query returned unexpected status {warm_status}"
        ));
    }

    let report = run_async_budget(
        SUITE,
        STUDIO_SEARCH_INDEX_STATUS_CASE,
        &perf_run_config(),
        || {
            let router = fixture.router();
            async move {
                let (status, payload) =
                    request_json(router, STUDIO_SEARCH_INDEX_STATUS_URI).await?;
                if status != StatusCode::OK {
                    return Err(anyhow!(
                        "unexpected status {status} for {STUDIO_SEARCH_INDEX_STATUS_URI}"
                    ));
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
    Ok(())
}
