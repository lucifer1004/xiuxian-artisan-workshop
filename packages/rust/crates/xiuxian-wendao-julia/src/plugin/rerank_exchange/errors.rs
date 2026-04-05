use xiuxian_wendao_core::repo_intelligence::RepoIntelligenceError;

pub(super) fn contract_decode_error(message: impl Into<String>) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "failed to decode Julia Arrow score rows from WendaoArrow `v1` contract: {}",
            message.into()
        ),
    }
}

pub(super) fn contract_request_error(message: impl Into<String>) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "failed to build WendaoArrow `v1` request batch: {}",
            message.into()
        ),
    }
}
