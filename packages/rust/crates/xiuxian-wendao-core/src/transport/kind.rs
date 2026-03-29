use serde::{Deserialize, Serialize};

/// Supported transport kinds for Wendao plugin-runtime data exchange.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginTransportKind {
    /// Apache Arrow Flight RPC.
    ArrowFlight,
    /// Arrow IPC over HTTP.
    #[default]
    ArrowIpcHttp,
    /// Local process invocation with Arrow IPC exchange.
    LocalProcessArrowIpc,
}
