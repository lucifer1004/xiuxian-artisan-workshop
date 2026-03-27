use serde::Deserialize;

use crate::zhenfa_router::native::WendaoJuliaDeploymentArtifactOutputFormat;

/// Query parameters for Studio Julia deployment artifact inspection.
#[derive(Debug, Default, Deserialize)]
pub struct JuliaDeploymentArtifactQuery {
    /// Optional response format. Defaults to structured JSON.
    #[serde(default)]
    pub format: Option<WendaoJuliaDeploymentArtifactOutputFormat>,
}
