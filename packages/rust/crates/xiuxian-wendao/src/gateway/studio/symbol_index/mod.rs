//! Background local-project symbol index coordinator for Studio.

mod state;
mod types;

pub(crate) use state::SymbolIndexCoordinator;
pub(crate) use types::{SymbolIndexPhase, SymbolIndexStatus};
