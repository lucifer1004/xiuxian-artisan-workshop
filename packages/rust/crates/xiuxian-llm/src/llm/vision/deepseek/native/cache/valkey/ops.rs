use std::sync::Arc;
use std::time::Duration;

use tracing::warn;

use super::super::super::super::util::sanitize_error_string;

pub(super) struct ValkeyOcrCache {
    pub(super) client: redis::Client,
    pub(super) key_prefix: Arc<str>,
    pub(super) ttl_secs: u64,
    pub(super) io_timeout_ms: u64,
}

impl ValkeyOcrCache {
    pub(super) fn get(&self, key: &str) -> Option<String> {
        let mut connection = match self.client.get_connection() {
            Ok(connection) => connection,
            Err(error) => {
                warn!(
                    event = "llm.vision.deepseek.valkey.get_connection_failed",
                    error = %sanitize_error_string(error),
                    "DeepSeek OCR Valkey read skipped because connection failed"
                );
                return None;
            }
        };
        self.apply_io_timeout(&connection);
        let full_key = self.key(key);
        let mut cmd = redis::cmd("GET");
        cmd.arg(full_key);
        match cmd.query::<Option<String>>(&mut connection) {
            Ok(value) => value,
            Err(error) => {
                warn!(
                    event = "llm.vision.deepseek.valkey.get_failed",
                    error = %sanitize_error_string(error),
                    "DeepSeek OCR Valkey read failed; falling back to local cache only"
                );
                None
            }
        }
    }

    pub(super) fn set(&self, key: &str, markdown: &str) -> bool {
        let mut connection = match self.client.get_connection() {
            Ok(connection) => connection,
            Err(error) => {
                warn!(
                    event = "llm.vision.deepseek.valkey.set_connection_failed",
                    error = %sanitize_error_string(error),
                    "DeepSeek OCR Valkey write skipped because connection failed"
                );
                return false;
            }
        };
        self.apply_io_timeout(&connection);
        let full_key = self.key(key);
        let mut cmd = redis::cmd("SETEX");
        cmd.arg(full_key).arg(self.ttl_secs).arg(markdown);
        if let Err(error) = cmd.query::<()>(&mut connection) {
            warn!(
                event = "llm.vision.deepseek.valkey.set_failed",
                error = %sanitize_error_string(error),
                "DeepSeek OCR Valkey write failed"
            );
            return false;
        }
        true
    }

    fn key(&self, key: &str) -> String {
        format!("{}:{key}", self.key_prefix)
    }

    fn apply_io_timeout(&self, connection: &redis::Connection) {
        let timeout = Duration::from_millis(self.io_timeout_ms);
        let _ = connection.set_read_timeout(Some(timeout));
        let _ = connection.set_write_timeout(Some(timeout));
    }
}
