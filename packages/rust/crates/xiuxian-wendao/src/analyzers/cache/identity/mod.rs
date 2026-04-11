mod classify;
mod fingerprint;

#[cfg(feature = "zhenfa-router")]
pub(crate) use classify::change_affects_analysis_identity;
#[cfg(feature = "zhenfa-router")]
pub(crate) use classify::{FingerprintMode, analysis_fingerprint_mode};
pub(crate) use fingerprint::collect_repository_analysis_identity;
