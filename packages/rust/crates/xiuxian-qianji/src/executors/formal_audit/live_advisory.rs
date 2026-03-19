//! Live LLM-backed advisory execution built on top of the advisory planning bridge.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use xiuxian_llm::llm::{ChatRequest, LlmClient};
use xiuxian_testing::{
    AdvisoryAuditExecutor, AdvisoryAuditRequest, FindingConfidence, FindingSeverity,
    RoleAuditFinding,
};
use xiuxian_zhenfa::{CognitiveDistribution, StreamProvider, ZhenfaPipeline};

use super::{QianjiAdvisoryAuditExecutor, QianjiAdvisoryExecutionPlan, QianjiAdvisoryRolePlan};

const DEFAULT_MODEL: &str = "gpt-5.4-mini";
const DEFAULT_TEMPERATURE: f32 = 0.1;

/// Live advisory executor that sends role plans through an `LlmClient`.
pub struct QianjiLlmAdvisoryAuditExecutor {
    /// Planning bridge reused for role resolution and snapshot assembly.
    pub planner: QianjiAdvisoryAuditExecutor,
    /// Client used to execute one critique per role.
    pub client: Arc<dyn LlmClient>,
    /// Default model used when the request does not override it.
    pub model: String,
    /// Temperature used for critique generation.
    pub temperature: f32,
    /// Whether to supervise streaming output through `ZhenfaPipeline`.
    pub enable_cognitive_supervision: bool,
    /// Coherence threshold used when cognitive supervision is enabled.
    pub cognitive_early_halt_threshold: f32,
}

impl QianjiLlmAdvisoryAuditExecutor {
    /// Create a new live advisory executor.
    #[must_use]
    pub fn new(
        planner: QianjiAdvisoryAuditExecutor,
        client: Arc<dyn LlmClient>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            planner,
            client,
            model: model.into(),
            temperature: DEFAULT_TEMPERATURE,
            enable_cognitive_supervision: false,
            cognitive_early_halt_threshold: 0.3,
        }
    }

    /// Override the critique temperature.
    #[must_use]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Enable cognitive supervision for streaming critiques.
    #[must_use]
    pub fn with_cognitive_supervision(mut self, early_halt_threshold: f32) -> Self {
        self.enable_cognitive_supervision = true;
        self.cognitive_early_halt_threshold = early_halt_threshold;
        self
    }

    async fn execute_role_critique(
        &self,
        request: &AdvisoryAuditRequest,
        role_plan: &QianjiAdvisoryRolePlan,
    ) -> Result<(String, Option<LiveCognitiveMetrics>)> {
        let request = ChatRequest::new(resolve_model(request, self.model.as_str()))
            .add_system_message(role_plan.rendered_prompt.clone())
            .add_user_message(live_advisory_instruction(request, role_plan))
            .with_temperature(self.temperature);

        if self.enable_cognitive_supervision {
            self.execute_with_cognitive_supervision(request).await
        } else {
            self.client
                .chat(request)
                .await
                .map(|response| (response, None))
                .map_err(Into::into)
        }
    }

    async fn execute_with_cognitive_supervision(
        &self,
        request: ChatRequest,
    ) -> Result<(String, Option<LiveCognitiveMetrics>)> {
        let mut pipeline = ZhenfaPipeline::with_options(
            resolve_provider(self.model.as_str()),
            true,
            true,
            self.cognitive_early_halt_threshold,
        );
        let mut stream = self.client.chat_stream(request).await?;
        let mut accumulated = String::new();
        let mut early_halt_reason = None;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            accumulated.push_str(&chunk);

            let synthetic_line = format!(
                r#"{{"type":"content_block_delta","index":0,"delta":{{"type":"text_delta","text":"{}"}}}}"#,
                chunk.replace('\\', "\\\\").replace('"', "\\\"")
            );
            if let Err(error) = pipeline.process_line(&synthetic_line) {
                early_halt_reason = Some(format!("pipeline violation: {error}"));
                break;
            }
            if pipeline.should_halt() {
                early_halt_reason = Some(format!(
                    "cognitive drift detected at coherence {:.2}",
                    pipeline.coherence_score()
                ));
                break;
            }
        }

        let _ = pipeline.finalize();

        Ok((
            accumulated,
            Some(LiveCognitiveMetrics {
                coherence: pipeline.coherence_score(),
                early_halt: early_halt_reason,
                distribution: pipeline.cognitive_distribution(),
            }),
        ))
    }
}

#[async_trait]
impl AdvisoryAuditExecutor for QianjiLlmAdvisoryAuditExecutor {
    async fn run(&self, request: AdvisoryAuditRequest) -> Result<Vec<RoleAuditFinding>> {
        let plan: QianjiAdvisoryExecutionPlan = self.planner.build_plan(&request).await?;
        let mut findings = QianjiAdvisoryAuditExecutor::findings_from_plan(&request, &plan);

        for (finding, role_plan) in findings.iter_mut().zip(&plan.roles) {
            let (critique_text, cognitive_metrics) =
                self.execute_role_critique(&request, role_plan).await?;
            apply_live_critique(finding, &critique_text, cognitive_metrics);
        }

        Ok(findings)
    }
}

#[derive(Debug, Clone)]
struct LiveCognitiveMetrics {
    coherence: f32,
    early_halt: Option<String>,
    distribution: CognitiveDistribution,
}

#[derive(Debug, Deserialize)]
struct LiveRoleCritiquePayload {
    summary: Option<String>,
    why_it_matters: Option<String>,
    remediation: Option<String>,
    severity: Option<String>,
    confidence: Option<String>,
    evidence_excerpt: Option<String>,
    good_example: Option<String>,
    bad_example: Option<String>,
}

fn resolve_model(request: &AdvisoryAuditRequest, default_model: &str) -> String {
    request
        .collection_context
        .labels
        .get("llm_model")
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            let trimmed = default_model.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .unwrap_or_else(|| DEFAULT_MODEL.to_string())
}

fn resolve_provider(model: &str) -> StreamProvider {
    let model_lower = model.to_ascii_lowercase();
    if model_lower.contains("claude") || model_lower.contains("anthropic") {
        StreamProvider::Claude
    } else if model_lower.contains("gemini") {
        StreamProvider::Gemini
    } else {
        StreamProvider::Codex
    }
}

fn live_advisory_instruction(
    request: &AdvisoryAuditRequest,
    role_plan: &QianjiAdvisoryRolePlan,
) -> String {
    let primary_title = request
        .findings
        .first()
        .map_or("contract review", |finding| finding.title.as_str());
    format!(
        "Review contract suite '{suite_id}' pack '{pack_id}' as role '{role_id}' ({persona_name}). \
Return one JSON object only with keys: summary, why_it_matters, remediation, severity, confidence, \
evidence_excerpt, good_example, bad_example. Use severity in [info, warning, error, critical] and \
confidence in [low, medium, high]. Focus on the primary issue '{primary_title}' and the evidence \
already provided in the system prompt. Do not wrap the JSON in markdown.",
        suite_id = request.suite_id,
        pack_id = request.pack_id,
        role_id = role_plan.role_id,
        persona_name = role_plan.persona_name,
    )
}

fn apply_live_critique(
    finding: &mut RoleAuditFinding,
    critique_text: &str,
    cognitive_metrics: Option<LiveCognitiveMetrics>,
) {
    finding
        .labels
        .insert("execution_mode".to_string(), "live_llm".to_string());
    finding.push_message_evidence(format!("Live advisory critique: {}", critique_text.trim()));

    if let Some(payload) = parse_live_payload(critique_text) {
        if let Some(summary) = payload.summary.filter(|value| !value.trim().is_empty()) {
            finding.summary = summary;
        }
        if let Some(why_it_matters) = payload
            .why_it_matters
            .filter(|value| !value.trim().is_empty())
        {
            finding.why_it_matters = why_it_matters;
        }
        if let Some(remediation) = payload.remediation.filter(|value| !value.trim().is_empty()) {
            finding.remediation = remediation;
        }
        if let Some(severity) = payload.severity.as_deref().and_then(parse_severity) {
            finding.severity = severity;
        }
        if let Some(confidence) = payload.confidence.as_deref().and_then(parse_confidence) {
            finding.confidence = confidence;
        }
        if let Some(evidence_excerpt) = payload
            .evidence_excerpt
            .filter(|value| !value.trim().is_empty())
        {
            finding.push_message_evidence(evidence_excerpt);
        }
        if let Some(good_example) = payload
            .good_example
            .filter(|value| !value.trim().is_empty())
        {
            finding.examples.good.push(good_example);
        }
        if let Some(bad_example) = payload.bad_example.filter(|value| !value.trim().is_empty()) {
            finding.examples.bad.push(bad_example);
        }
    }

    if let Some(metrics) = cognitive_metrics {
        finding.labels.insert(
            "cognitive_coherence".to_string(),
            format!("{:.3}", metrics.coherence),
        );
        finding
            .labels
            .insert("cognitive_monitoring".to_string(), "enabled".to_string());
        if let Some(reason) = metrics.early_halt.as_ref() {
            finding
                .labels
                .insert("cognitive_early_halt".to_string(), "true".to_string());
            finding.push_message_evidence(reason.clone());
        }
        finding.push_message_evidence(format!(
            "Cognitive distribution meta={:.3}, operational={:.3}, epistemic={:.3}, instrumental={:.3}, balance={:.3}, uncertainty_ratio={:.3}",
            metrics.distribution.meta,
            metrics.distribution.operational,
            metrics.distribution.epistemic,
            metrics.distribution.instrumental,
            metrics.distribution.balance(),
            metrics.distribution.uncertainty_ratio(),
        ));
    }
}

fn parse_live_payload(critique_text: &str) -> Option<LiveRoleCritiquePayload> {
    serde_json::from_str::<LiveRoleCritiquePayload>(critique_text.trim())
        .ok()
        .or_else(|| {
            let start = critique_text.find('{')?;
            let end = critique_text.rfind('}')?;
            serde_json::from_str::<LiveRoleCritiquePayload>(&critique_text[start..=end]).ok()
        })
}

fn parse_severity(raw: &str) -> Option<FindingSeverity> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "info" => Some(FindingSeverity::Info),
        "warning" | "warn" => Some(FindingSeverity::Warning),
        "error" => Some(FindingSeverity::Error),
        "critical" => Some(FindingSeverity::Critical),
        _ => None,
    }
}

fn parse_confidence(raw: &str) -> Option<FindingConfidence> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "high" => Some(FindingConfidence::High),
        "medium" | "med" => Some(FindingConfidence::Medium),
        "low" => Some(FindingConfidence::Low),
        _ => None,
    }
}
