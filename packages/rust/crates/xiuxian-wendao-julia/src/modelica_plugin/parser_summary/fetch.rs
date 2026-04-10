use xiuxian_wendao_core::repo_intelligence::{RegisteredRepository, RepoIntelligenceError};

use super::contract::{
    ModelicaParserSummaryRequestRow, build_modelica_parser_summary_request_batch,
    decode_modelica_parser_file_summary, decode_modelica_parser_summary_response_rows,
};
use super::transport::{
    ParserSummaryRouteKind, build_modelica_parser_summary_flight_transport_client,
    process_modelica_parser_summary_flight_batches_for_repository,
};
use super::types::ModelicaParserFileSummary;

pub(crate) async fn fetch_modelica_parser_file_summary_for_repository(
    repository: &RegisteredRepository,
    source_id: &str,
    source_text: &str,
) -> Result<ModelicaParserFileSummary, RepoIntelligenceError> {
    let batch = build_modelica_parser_summary_request_batch(&[ModelicaParserSummaryRequestRow {
        request_id: format!("modelica-file-summary:{source_id}"),
        source_id: source_id.to_string(),
        source_text: source_text.to_string(),
    }])?;
    let response_batches = process_modelica_parser_summary_flight_batches_for_repository(
        repository,
        ParserSummaryRouteKind::FileSummary,
        &[batch],
    )
    .await?;
    let rows = decode_modelica_parser_summary_response_rows(response_batches.as_slice())?;
    decode_modelica_parser_file_summary(ParserSummaryRouteKind::FileSummary, rows.as_slice())
}

pub(crate) fn fetch_modelica_parser_file_summary_blocking_for_repository(
    repository: &RegisteredRepository,
    source_id: &str,
    source_text: &str,
) -> Result<ModelicaParserFileSummary, RepoIntelligenceError> {
    let repository = repository.clone();
    let source_id = source_id.to_string();
    let source_text = source_text.to_string();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to build Modelica parser-summary runtime for repo `{}`: {error}",
                    repository.id,
                ),
            })?;
        runtime.block_on(fetch_modelica_parser_file_summary_for_repository(
            &repository,
            &source_id,
            &source_text,
        ))
    })
    .join()
    .map_err(|panic_payload| RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "Modelica parser-summary file-summary thread panicked: {}",
            panic_payload_message(&panic_payload),
        ),
    })?
}

pub(crate) fn validate_modelica_parser_summary_preflight_for_repository(
    repository: &RegisteredRepository,
) -> Result<(), RepoIntelligenceError> {
    let _client = build_modelica_parser_summary_flight_transport_client(
        repository,
        ParserSummaryRouteKind::FileSummary,
    )?;
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
