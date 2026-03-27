//! Generic Arrow IPC codec and Arrow-over-HTTP transport helpers.

mod client;
mod codec;
mod config;
mod error;

pub use client::ArrowTransportClient;
pub use codec::{
    attach_record_batch_metadata, attach_record_batch_trace_id, decode_record_batches_ipc,
    encode_record_batch_ipc, encode_record_batches_ipc,
};
pub use config::{
    ARROW_TRANSPORT_CONTENT_TYPE, ARROW_TRANSPORT_DEFAULT_BASE_URL,
    ARROW_TRANSPORT_DEFAULT_HEALTH_ROUTE, ARROW_TRANSPORT_DEFAULT_ROUTE,
    ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION, ARROW_TRANSPORT_SCHEMA_VERSION_METADATA_KEY,
    ARROW_TRANSPORT_TRACE_ID_METADATA_KEY, ArrowTransportConfig, ArrowTransportConfigError,
};
pub use error::ArrowTransportError;

#[cfg(test)]
mod tests;
