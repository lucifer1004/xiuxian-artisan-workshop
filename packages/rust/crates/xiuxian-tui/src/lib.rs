//! xiuxian-tui - Rust-driven TUI engine for Omni Dev Fusion
//!
//! Provides terminal UI rendering with foldable panels and event-driven updates.
//! Integrates with xiuxian-event for reactive state management.
pub mod cli_args;
pub mod components;
pub mod demo_cli_args;
pub mod event;
pub mod renderer;
/// Runtime bootstrap utilities for logger initialization and app lifecycle.
pub mod runtime;
pub mod socket;
pub mod state;

pub use components::{FoldablePanel, PanelState, TuiApp};
pub use event::{Event, EventHandler, TuiEvent};
pub use renderer::TuiRenderer;
pub use runtime::{init_logger, run_tui};
pub use socket::{SocketClient, SocketEvent, SocketServer};
pub use state::{
    AppState, ExecutionState, LogWindow, MAX_LOG_LINES, PanelType, ReceivedEvent, TaskItem,
    TaskStatus,
};
