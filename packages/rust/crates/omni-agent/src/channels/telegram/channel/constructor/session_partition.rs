use std::sync::PoisonError;

use super::super::TelegramSessionPartition;
use super::super::state::TelegramChannel;

impl TelegramChannel {
    /// Current session partition mode used by this channel.
    pub fn session_partition(&self) -> TelegramSessionPartition {
        *self
            .session_partition
            .read()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Update session partition mode at runtime.
    pub fn set_session_partition(&self, mode: TelegramSessionPartition) {
        *self
            .session_partition
            .write()
            .unwrap_or_else(PoisonError::into_inner) = mode;
    }
}
