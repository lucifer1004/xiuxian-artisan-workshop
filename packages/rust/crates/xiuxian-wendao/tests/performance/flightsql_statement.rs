use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use arrow::array::Array;
use arrow::record_batch::RecordBatch;
use arrow::util::display::array_value_to_string;
use arrow_flight::decode::FlightRecordBatchStream;
use arrow_flight::flight_service_server::FlightService;
use arrow_flight::sql::{CommandStatementQuery, ProstMessageExt};
use arrow_flight::{FlightData, FlightDescriptor};
use prost::Message;
use serial_test::file_serial;
use tokio_stream::StreamExt;
use tonic::Request;
use xiuxian_testing::{
    PerfBudget, PerfReport, PerfRunConfig, assert_perf_budget, run_async_budget,
};
use xiuxian_vector::SearchEngineContext;
use xiuxian_wendao::duckdb::ParquetQueryEngine;
use xiuxian_wendao::gateway::studio::perf_support::{
    GatewayPerfFixture, prepare_gateway_perf_fixture_with_julia_parser_summary_transport,
};
use xiuxian_wendao::search::queries::flightsql::{
    StudioFlightSqlService, build_studio_flightsql_service,
};
use xiuxian_wendao::search::{SearchCorpusKind, SearchRepoCorpusSnapshotRecord};
use xiuxian_wendao::set_link_graph_wendao_config_override;

use super::support::{env_f64, env_u64, env_usize};

const SUITE: &str = "xiuxian-wendao/perf";
const DATAFUSION_CASE: &str = "flightsql_statement_datafusion_p95";
const DUCKDB_CASE: &str = "flightsql_statement_duckdb_p95";
const REPO_ID: &str = "gateway-sync";
const EXPECTED_RESULT_ROWS: usize = 1;
const EXPECTED_PATH: &str = "src/GatewaySyncPkg.jl";

#[tokio::test(flavor = "current_thread")]
#[file_serial(wendao_flightsql_statement_perf_gate)]
async fn flightsql_statement_duckdb_vs_datafusion_p95_gate() -> Result<(), String> {
    let datafusion_override = write_runtime_override("datafusion", false)?;
    set_link_graph_wendao_config_override(datafusion_override.to_string_lossy().as_ref());
    let datafusion_fixture = prepare_gateway_perf_fixture_with_julia_parser_summary_transport()
        .await
        .map_err(|error| format!("prepare DataFusion FlightSQL perf fixture: {error}"))?;
    let datafusion_service = build_flightsql_service(&datafusion_fixture);
    let datafusion_probe = build_direct_engine_probe(&datafusion_fixture)?;

    let duckdb_override = write_runtime_override("duckdb", true)?;
    set_link_graph_wendao_config_override(duckdb_override.to_string_lossy().as_ref());
    let duckdb_fixture = prepare_gateway_perf_fixture_with_julia_parser_summary_transport()
        .await
        .map_err(|error| format!("prepare DuckDB FlightSQL perf fixture: {error}"))?;
    let duckdb_service = build_flightsql_service(&duckdb_fixture);
    let duckdb_probe = build_direct_engine_probe(&duckdb_fixture)?;

    let query = repo_content_chunk_query(REPO_ID);
    let config = flightsql_perf_config();

    let mut datafusion_report = run_async_budget(SUITE, DATAFUSION_CASE, &config, || {
        let service = datafusion_service.clone();
        let query = query.clone();
        async move { execute_statement_once(&service, query.as_str()).await }
    })
    .await;
    datafusion_report.add_metadata("engine", "datafusion");
    datafusion_report.add_metadata("protocol", "flightsql_statement");
    datafusion_report.add_metadata("repo_id", REPO_ID);
    datafusion_report.add_metadata("query", query.as_str());
    let datafusion_breakdown = capture_statement_phase_breakdown(
        &datafusion_service,
        &datafusion_probe,
        query.as_str(),
        &config,
    )
    .await?;
    datafusion_breakdown.write_metadata(&mut datafusion_report, "phase");
    persist_augmented_report(&datafusion_report)?;

    let mut duckdb_report = run_async_budget(SUITE, DUCKDB_CASE, &config, || {
        let service = duckdb_service.clone();
        let query = query.clone();
        async move { execute_statement_once(&service, query.as_str()).await }
    })
    .await;
    duckdb_report.add_metadata("engine", "duckdb");
    duckdb_report.add_metadata("protocol", "flightsql_statement");
    duckdb_report.add_metadata("repo_id", REPO_ID);
    duckdb_report.add_metadata("query", query.as_str());
    let duckdb_breakdown =
        capture_statement_phase_breakdown(&duckdb_service, &duckdb_probe, query.as_str(), &config)
            .await?;
    duckdb_breakdown.write_metadata(&mut duckdb_report, "phase");
    persist_augmented_report(&duckdb_report)?;

    let error_budget = PerfBudget {
        max_error_rate: Some(0.0),
        ..PerfBudget::new()
    };
    assert_perf_budget(&datafusion_report, &error_budget);
    assert_perf_budget(&duckdb_report, &error_budget);
    assert_duckdb_p95_ratio(&datafusion_report, &duckdb_report);

    println!(
        "flightsql_statement_perf_gate: datafusion_p95_ms={:.3}, duckdb_p95_ms={:.3}, ratio={:.3}, datafusion_phase_direct_engine_p95_ms={:.3}, datafusion_phase_get_flight_info_p95_ms={:.3}, datafusion_phase_do_get_collect_p95_ms={:.3}, datafusion_phase_decode_p95_ms={:.3}, duckdb_phase_direct_engine_p95_ms={:.3}, duckdb_phase_get_flight_info_p95_ms={:.3}, duckdb_phase_do_get_collect_p95_ms={:.3}, duckdb_phase_decode_p95_ms={:.3}, datafusion_report={:?}, duckdb_report={:?}",
        datafusion_report.quantiles.p95_ms,
        duckdb_report.quantiles.p95_ms,
        p95_ratio(&datafusion_report, &duckdb_report),
        datafusion_breakdown.direct_engine_query.p95_ms,
        datafusion_breakdown.get_flight_info.p95_ms,
        datafusion_breakdown.do_get_collect.p95_ms,
        datafusion_breakdown.decode.p95_ms,
        duckdb_breakdown.direct_engine_query.p95_ms,
        duckdb_breakdown.get_flight_info.p95_ms,
        duckdb_breakdown.do_get_collect.p95_ms,
        duckdb_breakdown.decode.p95_ms,
        datafusion_report.report_path,
        duckdb_report.report_path
    );

    set_link_graph_wendao_config_override(datafusion_override.to_string_lossy().as_ref());
    Ok(())
}

fn build_flightsql_service(fixture: &GatewayPerfFixture) -> StudioFlightSqlService {
    build_studio_flightsql_service(fixture.state().studio.search_plane_service())
}

fn build_direct_engine_probe(fixture: &GatewayPerfFixture) -> Result<DirectEngineProbe, String> {
    let storage_root = fixture_storage_root(fixture.root())?;
    let snapshot_path = storage_root
        .join("_runtime")
        .join("repo_corpus")
        .join("snapshot.json");
    let snapshot = fs::read_to_string(snapshot_path.as_path()).map_err(|error| {
        format!(
            "read repo corpus snapshot `{}`: {error}",
            snapshot_path.display()
        )
    })?;
    let snapshot: SearchRepoCorpusSnapshotRecord = serde_json::from_str(snapshot.as_str())
        .map_err(|error| {
            format!(
                "decode repo corpus snapshot `{}`: {error}",
                snapshot_path.display()
            )
        })?;
    let table_name = snapshot
        .records
        .into_iter()
        .find_map(|record| {
            (record.corpus == SearchCorpusKind::RepoContentChunk && record.repo_id == REPO_ID)
                .then_some(record.publication)
                .flatten()
                .map(|publication| publication.table_name)
        })
        .ok_or_else(|| {
            format!(
                "missing repo content publication record for repo `{REPO_ID}` in `{}`",
                snapshot_path.display()
            )
        })?;
    let parquet_path = storage_root
        .join(SearchCorpusKind::RepoContentChunk.as_str())
        .join("parquet")
        .join(format!("{table_name}.parquet"));
    if !parquet_path.exists() {
        return Err(format!(
            "missing repo content parquet fixture `{}`",
            parquet_path.display()
        ));
    }
    let engine = ParquetQueryEngine::configured(SearchEngineContext::new())
        .map_err(|error| format!("configure direct parquet query engine: {error}"))?;
    Ok(DirectEngineProbe {
        engine,
        query: source_table_query(table_name.as_str()),
        parquet_path,
        table_name,
    })
}

fn fixture_storage_root(project_root: &Path) -> Result<PathBuf, String> {
    let data_home = std::env::var_os("PRJ_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| fallback_data_root());
    let data_home = if data_home.is_absolute() {
        data_home
    } else {
        project_root.join(data_home)
    };
    Ok(data_home.join("wendao").join("search_plane").join(
        blake3::hash(project_root.to_string_lossy().as_bytes())
            .to_hex()
            .to_string(),
    ))
}

fn fallback_data_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../.data")
        .to_path_buf()
}

fn flightsql_perf_config() -> PerfRunConfig {
    PerfRunConfig {
        warmup_samples: env_usize("XIUXIAN_WENDAO_PERF_FLIGHTSQL_STATEMENT_WARMUP", 2),
        samples: env_usize("XIUXIAN_WENDAO_PERF_FLIGHTSQL_STATEMENT_SAMPLES", 10),
        timeout_ms: env_u64("XIUXIAN_WENDAO_PERF_FLIGHTSQL_STATEMENT_TIMEOUT_MS", 2_500),
        concurrency: 1,
    }
}

async fn execute_statement_once(
    service: &StudioFlightSqlService,
    query: &str,
) -> Result<(), String> {
    let descriptor = FlightDescriptor::new_cmd(
        CommandStatementQuery {
            query: query.to_string(),
            transaction_id: None,
        }
        .as_any()
        .encode_to_vec(),
    );
    let flight_info = FlightService::get_flight_info(service, Request::new(descriptor))
        .await
        .map_err(|error| format!("FlightSQL get_flight_info_statement failed: {error}"))?
        .into_inner();
    let ticket = flight_info
        .endpoint
        .first()
        .and_then(|endpoint| endpoint.ticket.clone())
        .ok_or_else(|| "FlightSQL statement flight info should expose one ticket".to_string())?;
    let frames = FlightService::do_get(service, Request::new(ticket))
        .await
        .map_err(|error| format!("FlightSQL do_get_statement failed: {error}"))?
        .into_inner()
        .collect::<Vec<Result<FlightData, tonic::Status>>>()
        .await;
    let batches = decode_flight_batches(frames).await?;
    let row_count = batches.iter().map(RecordBatch::num_rows).sum::<usize>();
    if row_count != EXPECTED_RESULT_ROWS {
        return Err(format!(
            "expected {EXPECTED_RESULT_ROWS} FlightSQL statement rows, got {row_count}"
        ));
    }
    let batch = batches
        .first()
        .ok_or_else(|| "FlightSQL statement should return at least one batch".to_string())?;
    let path = string_value(batch, "path", 0)?;
    if path != EXPECTED_PATH {
        return Err(format!(
            "expected first FlightSQL statement path `{EXPECTED_PATH}`, got `{path}`"
        ));
    }
    Ok(())
}

async fn capture_statement_phase_breakdown(
    service: &StudioFlightSqlService,
    direct_engine_probe: &DirectEngineProbe,
    query: &str,
    config: &PerfRunConfig,
) -> Result<StatementPhaseBreakdown, String> {
    let config = config.normalized();
    for _ in 0..config.warmup_samples {
        for _ in 0..config.concurrency {
            let _ = measure_statement_once(service, direct_engine_probe, query).await?;
        }
    }

    let mut samples = Vec::with_capacity(config.samples.saturating_mul(config.concurrency));
    for _ in 0..config.samples {
        for _ in 0..config.concurrency {
            samples.push(measure_statement_once(service, direct_engine_probe, query).await?);
        }
    }
    Ok(StatementPhaseBreakdown::from_samples(&samples))
}

async fn measure_statement_once(
    service: &StudioFlightSqlService,
    direct_engine_probe: &DirectEngineProbe,
    query: &str,
) -> Result<StatementPhaseTiming, String> {
    let descriptor = FlightDescriptor::new_cmd(
        CommandStatementQuery {
            query: query.to_string(),
            transaction_id: None,
        }
        .as_any()
        .encode_to_vec(),
    );
    let flight_info_started = Instant::now();
    let flight_info = FlightService::get_flight_info(service, Request::new(descriptor))
        .await
        .map_err(|error| format!("FlightSQL get_flight_info_statement failed: {error}"))?
        .into_inner();
    let get_flight_info_ms = duration_ms(flight_info_started.elapsed());
    let ticket = flight_info
        .endpoint
        .first()
        .and_then(|endpoint| endpoint.ticket.clone())
        .ok_or_else(|| "FlightSQL statement flight info should expose one ticket".to_string())?;
    let do_get_started = Instant::now();
    let frames = FlightService::do_get(service, Request::new(ticket))
        .await
        .map_err(|error| format!("FlightSQL do_get_statement failed: {error}"))?
        .into_inner()
        .collect::<Vec<Result<FlightData, tonic::Status>>>()
        .await;
    let do_get_collect_ms = duration_ms(do_get_started.elapsed());
    let decode_started = Instant::now();
    let batches = decode_flight_batches(frames).await?;
    let decode_ms = duration_ms(decode_started.elapsed());
    let validate_started = Instant::now();
    validate_statement_batches(&batches)?;
    let validate_ms = duration_ms(validate_started.elapsed());
    let direct_engine_query_ms = direct_engine_probe.query_once().await?;
    Ok(StatementPhaseTiming {
        direct_engine_query_ms,
        get_flight_info_ms,
        do_get_collect_ms,
        decode_ms,
        validate_ms,
    })
}

async fn decode_flight_batches(
    frames: Vec<Result<FlightData, tonic::Status>>,
) -> Result<Vec<RecordBatch>, String> {
    let batch_stream = FlightRecordBatchStream::new_from_flight_data(tokio_stream::iter(
        frames
            .into_iter()
            .map(|frame| frame.map_err(arrow_flight::error::FlightError::from)),
    ));
    let decoded = batch_stream.collect::<Vec<_>>().await;
    decoded
        .into_iter()
        .map(|batch| batch.map_err(|error| format!("decode FlightSQL statement batches: {error}")))
        .collect()
}

fn validate_statement_batches(batches: &[RecordBatch]) -> Result<(), String> {
    let row_count = batches.iter().map(RecordBatch::num_rows).sum::<usize>();
    if row_count != EXPECTED_RESULT_ROWS {
        return Err(format!(
            "expected {EXPECTED_RESULT_ROWS} FlightSQL statement rows, got {row_count}"
        ));
    }
    let batch = batches
        .first()
        .ok_or_else(|| "FlightSQL statement should return at least one batch".to_string())?;
    let path = string_value(batch, "path", 0)?;
    if path != EXPECTED_PATH {
        return Err(format!(
            "expected first FlightSQL statement path `{EXPECTED_PATH}`, got `{path}`"
        ));
    }
    Ok(())
}

fn duration_ms(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}

fn string_value(
    batch: &RecordBatch,
    column_name: &str,
    row_index: usize,
) -> Result<String, String> {
    let column = batch
        .column_by_name(column_name)
        .ok_or_else(|| format!("missing FlightSQL column `{column_name}`"))?;
    if column.is_null(row_index) {
        return Err(format!(
            "FlightSQL column `{column_name}` at row {row_index} should not be null"
        ));
    }
    array_value_to_string(column.as_ref(), row_index).map_err(|error| {
        format!("decode FlightSQL column `{column_name}` at row {row_index}: {error}")
    })
}

fn repo_content_chunk_query(repo_id: &str) -> String {
    source_table_query(repo_content_chunk_source_table_name(repo_id).as_str())
}

fn repo_content_chunk_source_table_name(repo_id: &str) -> String {
    format!(
        "repo_content_chunk_repo_{}",
        blake3::hash(repo_id.as_bytes()).to_hex()
    )
}

fn source_table_query(table_name: &str) -> String {
    format!("SELECT path, line_number FROM {table_name} ORDER BY path, line_number LIMIT 1")
}

fn write_runtime_override(label: &str, duckdb_enabled: bool) -> Result<PathBuf, String> {
    let root = runtime_override_root()
        .join("xiuxian-wendao")
        .join("flightsql-statement");
    let config_root = root.join(label);
    fs::create_dir_all(&config_root).map_err(|error| {
        format!(
            "create FlightSQL perf override dir `{}`: {error}",
            config_root.display()
        )
    })?;
    let config_path = config_root.join("wendao.toml");
    let body = if duckdb_enabled {
        let temp_directory = config_root.join("duckdb-tmp");
        fs::create_dir_all(&temp_directory).map_err(|error| {
            format!(
                "create DuckDB temp directory `{}`: {error}",
                temp_directory.display()
            )
        })?;
        format!(
            "[search.duckdb]\nenabled = true\ndatabase_path = \":memory:\"\ntemp_directory = \"{}\"\nthreads = 2\nmaterialize_threshold_rows = 4096\nprefer_virtual_arrow = true\n",
            temp_directory.display()
        )
    } else {
        "[search.duckdb]\nenabled = false\n".to_string()
    };
    fs::write(&config_path, body).map_err(|error| {
        format!(
            "write FlightSQL perf runtime override `{}`: {error}",
            config_path.display()
        )
    })?;
    Ok(config_path)
}

fn runtime_override_root() -> PathBuf {
    std::env::var_os("PRJ_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| fallback_runtime_root())
}

fn fallback_runtime_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../.run")
        .to_path_buf()
}

fn assert_duckdb_p95_ratio(datafusion: &PerfReport, duckdb: &PerfReport) {
    let ratio = p95_ratio(datafusion, duckdb);
    let max_ratio = env_f64(
        "XIUXIAN_WENDAO_PERF_FLIGHTSQL_STATEMENT_MAX_P95_RATIO",
        if std::env::var_os("CI").is_some() {
            2.0
        } else {
            1.8
        },
    );
    assert!(
        ratio <= max_ratio,
        "DuckDB FlightSQL statement p95 ratio exceeded budget: duckdb_p95_ms={:.3}, datafusion_p95_ms={:.3}, ratio={:.3}, budget<={:.3}, duckdb_report={:?}, datafusion_report={:?}",
        duckdb.quantiles.p95_ms,
        datafusion.quantiles.p95_ms,
        ratio,
        max_ratio,
        duckdb.report_path,
        datafusion.report_path
    );
}

fn p95_ratio(datafusion: &PerfReport, duckdb: &PerfReport) -> f64 {
    let baseline = datafusion.quantiles.p95_ms.max(0.001);
    duckdb.quantiles.p95_ms / baseline
}

fn persist_augmented_report(report: &PerfReport) -> Result<(), String> {
    let Some(path) = report.report_path.as_deref() else {
        return Ok(());
    };
    let payload = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("serialize augmented perf report `{path}`: {error}"))?;
    fs::write(path, payload)
        .map_err(|error| format!("rewrite augmented perf report `{path}`: {error}"))?;
    Ok(())
}

struct DirectEngineProbe {
    engine: ParquetQueryEngine,
    table_name: String,
    parquet_path: PathBuf,
    query: String,
}

impl DirectEngineProbe {
    async fn query_once(&self) -> Result<f64, String> {
        self.engine
            .ensure_parquet_table_registered(self.table_name.as_str(), self.parquet_path.as_path())
            .await
            .map_err(|error| {
                format!(
                    "register direct parquet table `{}`: {error}",
                    self.table_name
                )
            })?;
        let started = Instant::now();
        let batches = self
            .engine
            .query_batches(self.query.as_str())
            .await
            .map_err(|error| format!("direct parquet query `{}` failed: {error}", self.query))?;
        let elapsed_ms = duration_ms(started.elapsed());
        validate_statement_batches(&batches)?;
        Ok(elapsed_ms)
    }
}

struct StatementPhaseTiming {
    direct_engine_query_ms: f64,
    get_flight_info_ms: f64,
    do_get_collect_ms: f64,
    decode_ms: f64,
    validate_ms: f64,
}

struct StatementPhaseBreakdown {
    direct_engine_query: PhaseSummary,
    get_flight_info: PhaseSummary,
    do_get_collect: PhaseSummary,
    decode: PhaseSummary,
    validate: PhaseSummary,
}

impl StatementPhaseBreakdown {
    fn from_samples(samples: &[StatementPhaseTiming]) -> Self {
        Self {
            direct_engine_query: PhaseSummary::from_ms(
                &samples
                    .iter()
                    .map(|sample| sample.direct_engine_query_ms)
                    .collect::<Vec<_>>(),
            ),
            get_flight_info: PhaseSummary::from_ms(
                &samples
                    .iter()
                    .map(|sample| sample.get_flight_info_ms)
                    .collect::<Vec<_>>(),
            ),
            do_get_collect: PhaseSummary::from_ms(
                &samples
                    .iter()
                    .map(|sample| sample.do_get_collect_ms)
                    .collect::<Vec<_>>(),
            ),
            decode: PhaseSummary::from_ms(
                &samples
                    .iter()
                    .map(|sample| sample.decode_ms)
                    .collect::<Vec<_>>(),
            ),
            validate: PhaseSummary::from_ms(
                &samples
                    .iter()
                    .map(|sample| sample.validate_ms)
                    .collect::<Vec<_>>(),
            ),
        }
    }

    fn write_metadata(&self, report: &mut PerfReport, prefix: &str) {
        self.direct_engine_query
            .write_metadata(report, prefix, "direct_engine_query");
        self.get_flight_info
            .write_metadata(report, prefix, "get_flight_info");
        self.do_get_collect
            .write_metadata(report, prefix, "do_get_collect");
        self.decode.write_metadata(report, prefix, "decode");
        self.validate.write_metadata(report, prefix, "validate");
    }
}

struct PhaseSummary {
    min_ms: f64,
    mean_ms: f64,
    max_ms: f64,
    p50_ms: f64,
    p95_ms: f64,
}

impl PhaseSummary {
    fn from_ms(samples: &[f64]) -> Self {
        if samples.is_empty() {
            return Self {
                min_ms: 0.0,
                mean_ms: 0.0,
                max_ms: 0.0,
                p50_ms: 0.0,
                p95_ms: 0.0,
            };
        }
        let mut sorted = samples.to_vec();
        sorted.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
        let mean_ms = sorted.iter().sum::<f64>() / sorted.len() as f64;
        Self {
            min_ms: sorted[0],
            mean_ms,
            max_ms: *sorted.last().unwrap_or(&0.0),
            p50_ms: percentile(&sorted, 50, 100),
            p95_ms: percentile(&sorted, 95, 100),
        }
    }

    fn write_metadata(&self, report: &mut PerfReport, prefix: &str, phase: &str) {
        report.add_metadata(
            format!("{prefix}_{phase}_min_ms"),
            format!("{:.3}", self.min_ms),
        );
        report.add_metadata(
            format!("{prefix}_{phase}_mean_ms"),
            format!("{:.3}", self.mean_ms),
        );
        report.add_metadata(
            format!("{prefix}_{phase}_max_ms"),
            format!("{:.3}", self.max_ms),
        );
        report.add_metadata(
            format!("{prefix}_{phase}_p50_ms"),
            format!("{:.3}", self.p50_ms),
        );
        report.add_metadata(
            format!("{prefix}_{phase}_p95_ms"),
            format!("{:.3}", self.p95_ms),
        );
    }
}

fn percentile(samples: &[f64], numerator: usize, denominator: usize) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let max_index = samples.len().saturating_sub(1);
    let rounded = max_index
        .saturating_mul(numerator)
        .saturating_add(denominator / 2)
        / denominator;
    samples[rounded.min(max_index)]
}
