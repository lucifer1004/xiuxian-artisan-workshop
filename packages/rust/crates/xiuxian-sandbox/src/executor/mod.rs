//! Sandbox execution backends.

pub mod nsjail;
pub mod seatbelt;
mod types;
pub use nsjail::NsJailExecutor;
pub use seatbelt::SeatbeltExecutor;
use types::execute_with_limits;
pub use types::{ExecutionResult, MountConfig, SandboxConfig, SandboxExecutor};
