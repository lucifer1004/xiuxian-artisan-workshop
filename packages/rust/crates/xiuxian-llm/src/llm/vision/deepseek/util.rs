use crate::llm::error::{LlmError, sanitize_user_visible};

pub(super) fn non_empty_env(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn internal_error(message: impl Into<String>) -> LlmError {
    LlmError::Internal {
        message: sanitize_user_visible(&message.into()),
    }
}

pub(super) fn sanitize_error_string(error: impl std::fmt::Display) -> String {
    sanitize_user_visible(error.to_string().as_str())
}
