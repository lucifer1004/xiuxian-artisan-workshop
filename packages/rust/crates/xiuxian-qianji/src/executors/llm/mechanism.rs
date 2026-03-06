use super::model::resolve_model_for_request;
use super::output::build_output_data;
use crate::contracts::{FlowInstruction, QianjiMechanism, QianjiOutput};
use crate::executors::annotation::ContextAnnotator;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use xiuxian_llm::llm::{ChatMessage, ChatRequest, LlmClient, MessageRole};

/// Mechanism responsible for performing LLM inference based on Qianhuan-orchestrated prompts.
pub struct LlmAnalyzer {
    /// Thread-safe client for LLM communication.
    pub client: Arc<dyn LlmClient>,
    /// Node-local context annotator used to generate system prompts.
    pub annotator: ContextAnnotator,
    /// Target model name.
    pub model: String,
    /// The output key to store the result.
    pub output_key: String,
    /// Whether to parse model output as JSON and store structured value.
    pub parse_json_output: bool,
    /// Whether to build a fallback shard plan from `repo_tree` when JSON parsing fails.
    pub fallback_repo_tree_on_parse_failure: bool,
}

#[async_trait]
impl QianjiMechanism for LlmAnalyzer {
    async fn execute(&self, context: &serde_json::Value) -> Result<QianjiOutput, String> {
        // 1. Generate system prompt using the embedded annotator
        let annotation_output = self.annotator.execute(context).await?;
        let Value::Object(mut data) = annotation_output.data else {
            return Err("LlmAnalyzer expected annotation output object".to_string());
        };

        let system_prompt = data
            .get(&self.annotator.output_key)
            .and_then(Value::as_str)
            .ok_or_else(|| {
                format!(
                    "LlmAnalyzer missing annotated prompt at key `{}`",
                    self.annotator.output_key
                )
            })?;

        // 2. Resolve user query
        let user_query = context
            .get("User_Intent")
            .or_else(|| context.get("query"))
            .or_else(|| context.get("raw_facts"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("Proceed with the analysis based on your mandate.");

        // 3. Prepare ChatRequest
        let request = ChatRequest {
            model: resolve_model_for_request(context, &self.model),
            messages: vec![
                ChatMessage {
                    role: MessageRole::System,
                    content: Some(system_prompt.to_string().into()),
                    ..ChatMessage::default()
                },
                ChatMessage {
                    role: MessageRole::User,
                    content: Some(user_query.to_string().into()),
                    ..ChatMessage::default()
                },
            ],
            temperature: Some(0.1),
            ..ChatRequest::default()
        };

        // 4. Execute LLM call
        let conclusion = self
            .client
            .chat(request)
            .await
            .map_err(|e| format!("LLM execution failed: {e}"))?;

        // 5. Build final output data
        let llm_data = build_output_data(
            &self.output_key,
            self.parse_json_output,
            self.fallback_repo_tree_on_parse_failure,
            context,
            conclusion,
        );

        // Merge LLM data into the annotation data (which includes persona_id, etc.)
        data.extend(llm_data);

        Ok(QianjiOutput {
            data: serde_json::Value::Object(data),
            instruction: FlowInstruction::Continue,
        })
    }

    fn weight(&self) -> f32 {
        3.0
    }
}
