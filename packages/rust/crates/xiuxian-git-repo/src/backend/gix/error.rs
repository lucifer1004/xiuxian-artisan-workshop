#[derive(Debug, Clone)]
pub(crate) struct BackendError {
    message: String,
}

impl BackendError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for BackendError {}

pub(super) fn error_message(error: impl std::fmt::Display) -> BackendError {
    BackendError::new(error.to_string())
}

pub(super) fn boxed_error(
    error: impl std::error::Error + Send + Sync + 'static,
) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(error)
}
