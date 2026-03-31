//! xiuxian-zhixing - The 'Unity of Knowledge and Action' logic layer.

/// Compile-time embedded resource tree rooted at `xiuxian-zhixing/resources`.
pub static RESOURCES: ::include_dir::Dir<'_> =
    ::include_dir::include_dir!("$CARGO_MANIFEST_DIR/resources");

/// Agenda domain models and task lifecycle logic.
pub mod agenda;
/// Alchemy-related processors and orchestration primitives.
pub mod alchemist;
/// Shared error types and crate-level `Result` alias.
pub mod error;
/// Core "Knowledge and Action Unity" orchestration.
pub mod heyi;
/// Journal domain model and parsing.
pub mod journal;
/// Storage backends for journals and agendas.
pub mod storage;

pub use agenda::AgendaEntry;
pub use error::{Error, Result};
pub use heyi::{
    ATTR_JOURNAL_CARRYOVER, ATTR_TIMER_RECIPIENT, ATTR_TIMER_REMINDED, ATTR_TIMER_SCHEDULED,
    ReminderQueueSettings, ReminderQueueStore, ReminderSignal, ZhixingHeyi,
};
pub use journal::JournalEntry;
