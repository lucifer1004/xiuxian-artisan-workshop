use reqwest::header::CONTENT_TYPE;

use super::config::ArrowTransportConfig;
use super::error::ArrowTransportError;
use super::{decode_record_batches_ipc, encode_record_batches_ipc};
use arrow::record_batch::RecordBatch;

const WENDAO_SCHEMA_VERSION_HEADER: &str = "x-wendao-schema-version";

/// HTTP client for Arrow IPC roundtrips against a WendaoArrow-compatible service.
#[derive(Clone)]
pub struct ArrowTransportClient {
    client: reqwest::Client,
    config: ArrowTransportConfig,
}

impl ArrowTransportClient {
    /// Create a new client using the timeout configured in
    /// [`ArrowTransportConfig`].
    ///
    /// # Errors
    ///
    /// Returns [`ArrowTransportError`] when the transport config is invalid or
    /// the underlying HTTP client cannot be constructed.
    pub fn new(config: ArrowTransportConfig) -> Result<Self, ArrowTransportError> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout())
            .build()
            .map_err(ArrowTransportError::BuildClient)?;
        Self::from_parts(client, config)
    }

    /// Create a client from an existing `reqwest::Client`.
    ///
    /// # Errors
    ///
    /// Returns [`ArrowTransportError`] when the transport config contains an
    /// invalid base URL or route.
    pub fn from_parts(
        client: reqwest::Client,
        config: ArrowTransportConfig,
    ) -> Result<Self, ArrowTransportError> {
        config.endpoint_url()?;
        config.health_url()?;
        Ok(Self { client, config })
    }

    /// Return the runtime config backing this client.
    #[must_use]
    pub fn config(&self) -> &ArrowTransportConfig {
        &self.config
    }

    /// Probe the remote health endpoint.
    ///
    /// # Errors
    ///
    /// Returns [`ArrowTransportError`] when the request fails or the endpoint
    /// does not respond with a success status.
    pub async fn check_health(&self) -> Result<(), ArrowTransportError> {
        let response = self
            .client
            .get(self.config.health_url()?)
            .send()
            .await
            .map_err(ArrowTransportError::Http)?;
        let response = ensure_success(response).await?;
        ensure_schema_version(response.headers(), self.config.schema_version())?;
        Ok(())
    }

    /// Send a single `RecordBatch` to the remote processor.
    ///
    /// # Errors
    ///
    /// Returns [`ArrowTransportError`] when request encoding, HTTP transport,
    /// or response decoding fails.
    pub async fn process_batch(
        &self,
        batch: &RecordBatch,
    ) -> Result<Vec<RecordBatch>, ArrowTransportError> {
        self.process_batches(std::slice::from_ref(batch)).await
    }

    /// Send multiple `RecordBatch` values to the remote processor.
    ///
    /// # Errors
    ///
    /// Returns [`ArrowTransportError`] when request encoding, HTTP transport,
    /// or response decoding fails.
    pub async fn process_batches(
        &self,
        batches: &[RecordBatch],
    ) -> Result<Vec<RecordBatch>, ArrowTransportError> {
        if batches.is_empty() {
            return Err(ArrowTransportError::EmptyRequest);
        }

        let payload = encode_record_batches_ipc(batches).map_err(ArrowTransportError::Encode)?;
        let response = self
            .client
            .post(self.config.endpoint_url()?)
            .header(CONTENT_TYPE, self.config.content_type())
            .header(WENDAO_SCHEMA_VERSION_HEADER, self.config.schema_version())
            .body(payload)
            .send()
            .await
            .map_err(ArrowTransportError::Http)?;
        let response = ensure_success(response).await?;

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("<missing>");
        if !content_type.starts_with(self.config.content_type()) {
            return Err(ArrowTransportError::UnexpectedContentType {
                expected: self.config.content_type().to_string(),
                found: content_type.to_string(),
            });
        }

        ensure_schema_version(response.headers(), self.config.schema_version())?;

        let body = response.bytes().await.map_err(ArrowTransportError::Http)?;
        decode_record_batches_ipc(body.as_ref()).map_err(ArrowTransportError::Decode)
    }
}

async fn ensure_success(
    response: reqwest::Response,
) -> Result<reqwest::Response, ArrowTransportError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let body = response.text().await.map_err(ArrowTransportError::Http)?;
    Err(ArrowTransportError::UnexpectedStatus { status, body })
}

fn ensure_schema_version(
    headers: &reqwest::header::HeaderMap,
    expected_schema_version: &str,
) -> Result<(), ArrowTransportError> {
    let observed = headers
        .get(WENDAO_SCHEMA_VERSION_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("<missing>");
    if observed == expected_schema_version {
        return Ok(());
    }

    Err(ArrowTransportError::UnexpectedSchemaVersion {
        expected: expected_schema_version.to_string(),
        found: observed.to_string(),
    })
}
