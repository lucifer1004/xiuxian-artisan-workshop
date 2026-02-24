use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct EmbedBatchResponse {
    pub(crate) vectors: Option<Vec<Vec<f32>>>,
}

#[derive(Deserialize)]
pub(crate) struct McpEmbedResult {
    #[serde(default)]
    pub(crate) success: bool,
    #[serde(default)]
    pub(crate) vectors: Vec<Vec<f32>>,
}
