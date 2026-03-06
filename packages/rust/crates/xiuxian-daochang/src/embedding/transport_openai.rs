use reqwest::Client;

pub(crate) async fn embed_openai_http(
    client: &Client,
    base_url: &str,
    texts: &[String],
    model: Option<&str>,
) -> Option<Vec<Vec<f32>>> {
    xiuxian_llm::embedding::openai_compat::embed_openai_compatible(client, base_url, texts, model)
        .await
}
