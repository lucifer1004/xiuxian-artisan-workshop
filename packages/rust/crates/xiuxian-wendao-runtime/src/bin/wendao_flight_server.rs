//! Runtime-owned Arrow Flight server binary for the stable Wendao query and rerank routes.

use std::io::{self, Write};
use std::sync::Arc;

use arrow_flight::flight_service_server::FlightServiceServer;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;
use xiuxian_vector::{
    LanceDataType, LanceField, LanceFloat64Array, LanceRecordBatch, LanceSchema, LanceStringArray,
};
use xiuxian_wendao_runtime::transport::{
    REPO_SEARCH_DOC_ID_COLUMN, REPO_SEARCH_LANGUAGE_COLUMN, REPO_SEARCH_PATH_COLUMN,
    REPO_SEARCH_SCORE_COLUMN, REPO_SEARCH_TITLE_COLUMN, WendaoFlightService,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bind_addr = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:0".to_string());
    let expected_schema_version = std::env::args().nth(2).unwrap_or_else(|| "v2".to_string());
    let rerank_dimension = std::env::args()
        .nth(3)
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(3);

    let listener = TcpListener::bind(bind_addr).await?;
    let address = listener.local_addr()?;
    let query_response_batch = LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new(REPO_SEARCH_DOC_ID_COLUMN, LanceDataType::Utf8, false),
            LanceField::new(REPO_SEARCH_PATH_COLUMN, LanceDataType::Utf8, false),
            LanceField::new(REPO_SEARCH_TITLE_COLUMN, LanceDataType::Utf8, false),
            LanceField::new(REPO_SEARCH_SCORE_COLUMN, LanceDataType::Float64, false),
            LanceField::new(REPO_SEARCH_LANGUAGE_COLUMN, LanceDataType::Utf8, false),
        ])),
        vec![
            Arc::new(LanceStringArray::from(vec!["doc-1"])),
            Arc::new(LanceStringArray::from(vec!["src/lib.rs"])),
            Arc::new(LanceStringArray::from(vec!["Repo Search Result"])),
            Arc::new(LanceFloat64Array::from(vec![0.91_f64])),
            Arc::new(LanceStringArray::from(vec!["rust"])),
        ],
    )?;
    writeln!(io::stdout(), "READY http://{address}")?;
    io::stdout().flush()?;

    let service = WendaoFlightService::new(
        expected_schema_version,
        query_response_batch,
        rerank_dimension,
    )?;

    Server::builder()
        .add_service(FlightServiceServer::new(service))
        .serve_with_incoming(TcpListenerStream::new(listener))
        .await?;

    Ok(())
}
