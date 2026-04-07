mod classify;
mod fingerprint;

pub(crate) use classify::{
    FingerprintMode, analysis_fingerprint_mode, change_affects_analysis_identity,
};
pub(crate) use fingerprint::collect_repository_analysis_identity;
