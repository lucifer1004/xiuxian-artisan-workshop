use xiuxian_wendao_core::repo_intelligence::{RegisteredRepository, RepoIntelligenceError};

use super::contract::{
    JuliaParserSummaryRequestRow, build_julia_parser_summary_request_batch,
    decode_julia_parser_file_summary, decode_julia_parser_root_summary,
    decode_julia_parser_summary_response_rows,
};
use super::transport::{
    ParserSummaryRouteKind, build_julia_parser_summary_flight_transport_client,
    process_julia_parser_summary_flight_batches_for_repository,
};
use super::types::{JuliaParserFileSummary, JuliaParserSourceSummary};

/// Build one parser-summary request, execute the configured Flight roundtrip,
/// and decode a Julia file summary.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the request cannot be materialized,
/// the repository does not declare a usable parser-summary client, the roundtrip
/// fails, or the response violates the staged contract.
pub(crate) async fn fetch_julia_parser_file_summary_for_repository(
    repository: &RegisteredRepository,
    source_id: &str,
    source_text: &str,
) -> Result<JuliaParserFileSummary, RepoIntelligenceError> {
    let batch = build_julia_parser_summary_request_batch(&[JuliaParserSummaryRequestRow {
        request_id: format!("julia-file-summary:{source_id}"),
        source_id: source_id.to_string(),
        source_text: source_text.to_string(),
    }])?;
    let response_batches = process_julia_parser_summary_flight_batches_for_repository(
        repository,
        ParserSummaryRouteKind::FileSummary,
        &[batch],
    )
    .await?;
    let rows = decode_julia_parser_summary_response_rows(response_batches.as_slice())?;
    decode_julia_parser_file_summary(ParserSummaryRouteKind::FileSummary, rows.as_slice())
}

/// Build one parser-summary request, execute the configured Flight roundtrip,
/// and decode a Julia root summary.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the request cannot be materialized,
/// the repository does not declare a usable parser-summary client, the roundtrip
/// fails, or the response violates the staged contract.
pub(crate) async fn fetch_julia_parser_root_summary_for_repository(
    repository: &RegisteredRepository,
    source_id: &str,
    source_text: &str,
) -> Result<JuliaParserSourceSummary, RepoIntelligenceError> {
    let batch = build_julia_parser_summary_request_batch(&[JuliaParserSummaryRequestRow {
        request_id: format!("julia-root-summary:{source_id}"),
        source_id: source_id.to_string(),
        source_text: source_text.to_string(),
    }])?;
    let response_batches = process_julia_parser_summary_flight_batches_for_repository(
        repository,
        ParserSummaryRouteKind::RootSummary,
        &[batch],
    )
    .await?;
    let rows = decode_julia_parser_summary_response_rows(response_batches.as_slice())?;
    decode_julia_parser_root_summary(ParserSummaryRouteKind::RootSummary, rows.as_slice())
}

pub(crate) fn fetch_julia_parser_file_summary_blocking_for_repository(
    repository: &RegisteredRepository,
    source_id: &str,
    source_text: &str,
) -> Result<JuliaParserFileSummary, RepoIntelligenceError> {
    let repository = repository.clone();
    let source_id = source_id.to_string();
    let source_text = source_text.to_string();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to build Julia parser-summary runtime for repo `{}`: {error}",
                    repository.id,
                ),
            })?;
        runtime.block_on(fetch_julia_parser_file_summary_for_repository(
            &repository,
            &source_id,
            &source_text,
        ))
    })
    .join()
    .map_err(|panic_payload| RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "Julia parser-summary file-summary thread panicked: {}",
            panic_payload_message(&panic_payload),
        ),
    })?
}

pub(crate) fn fetch_julia_parser_root_summary_blocking_for_repository(
    repository: &RegisteredRepository,
    source_id: &str,
    source_text: &str,
) -> Result<JuliaParserSourceSummary, RepoIntelligenceError> {
    let repository = repository.clone();
    let source_id = source_id.to_string();
    let source_text = source_text.to_string();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to build Julia parser-summary runtime for repo `{}`: {error}",
                    repository.id,
                ),
            })?;
        runtime.block_on(fetch_julia_parser_root_summary_for_repository(
            &repository,
            &source_id,
            &source_text,
        ))
    })
    .join()
    .map_err(|panic_payload| RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "Julia parser-summary root-summary thread panicked: {}",
            panic_payload_message(&panic_payload),
        ),
    })?
}

pub(crate) fn validate_julia_parser_summary_preflight_for_repository(
    repository: &RegisteredRepository,
) -> Result<(), RepoIntelligenceError> {
    for route_kind in [
        ParserSummaryRouteKind::FileSummary,
        ParserSummaryRouteKind::RootSummary,
    ] {
        let _client = build_julia_parser_summary_flight_transport_client(repository, route_kind)?;
    }
    Ok(())
}

fn panic_payload_message(panic_payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic_payload.downcast_ref::<&'static str>() {
        (*message).to_string()
    } else if let Some(message) = panic_payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/plugin/parser_summary/fetch.rs"]
mod tests;
