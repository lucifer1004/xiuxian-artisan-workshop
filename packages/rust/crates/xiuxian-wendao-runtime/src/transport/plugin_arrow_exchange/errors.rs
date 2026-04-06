use crate::transport::contract::DEFAULT_FLIGHT_SCHEMA_VERSION;
use xiuxian_wendao_core::repo_intelligence::RepoIntelligenceError;

pub(super) fn contract_decode_error(message: impl Into<String>) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "failed to decode plugin Arrow score rows from WendaoArrow `{DEFAULT_FLIGHT_SCHEMA_VERSION}` contract: {}",
            message.into()
        ),
    }
}

pub(super) fn contract_request_error(message: impl Into<String>) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "failed to build WendaoArrow `{DEFAULT_FLIGHT_SCHEMA_VERSION}` request batch: {}",
            message.into()
        ),
    }
}

pub(super) fn contract_response_error(message: impl Into<String>) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "WendaoArrow response contract `{DEFAULT_FLIGHT_SCHEMA_VERSION}` violated: {}",
            message.into()
        ),
    }
}
