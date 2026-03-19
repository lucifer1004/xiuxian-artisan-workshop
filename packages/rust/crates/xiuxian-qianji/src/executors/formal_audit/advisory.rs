//! Advisory-audit bridge from `xiuxian-testing` into `Qianji` and `Qianhuan`.

use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use xiuxian_qianhuan::{
    InjectionPolicy, InjectionSnapshot, PersonaProfile, PersonaRegistry, PromptContextBlock,
    PromptContextCategory, PromptContextSource, RoleMixProfile, RoleMixRole,
    ThousandFacesOrchestrator,
};
use xiuxian_testing::{
    AdvisoryAuditExecutor, AdvisoryAuditRequest, ArtifactKind, ContractFinding, EvidenceKind,
    FindingEvidence, RoleAuditFinding,
};

const DEFAULT_ROLE_ID: &str = "strict_teacher";

/// Planned advisory execution state for one resolved role.
#[derive(Debug, Clone, PartialEq)]
pub struct QianjiAdvisoryRolePlan {
    /// Stable role identifier requested by the contract runner.
    pub role_id: String,
    /// Friendly persona name resolved from the `Qianhuan` registry.
    pub persona_name: String,
    /// Typed `Qianhuan` injection snapshot prepared for this role.
    pub snapshot: InjectionSnapshot,
    /// Fully rendered system prompt snapshot prepared for later live execution.
    pub rendered_prompt: String,
}

/// Planned multi-role advisory execution payload.
#[derive(Debug, Clone, PartialEq)]
pub struct QianjiAdvisoryExecutionPlan {
    /// Stable suite identifier from the contract runner.
    pub suite_id: String,
    /// Rule-pack identifier under review.
    pub pack_id: String,
    /// Resolved role mix for this advisory pass.
    pub role_mix: RoleMixProfile,
    /// Per-role snapshot plan.
    pub roles: Vec<QianjiAdvisoryRolePlan>,
}

/// Qianji-side advisory executor scaffold backed by `Qianhuan` persona resolution.
///
/// This executor does not perform live LLM critique yet. Instead, it converts a
/// `xiuxian-testing` `AdvisoryAuditRequest` into:
/// - a `RoleMixProfile`
/// - typed `InjectionSnapshot` values for each role
/// - normalized `RoleAuditFinding` values that preserve deterministic evidence and trace context
///
/// The resulting bridge is immediately useful for testing and knowledge export while keeping the
/// future live `formal_audit + Zhenfa` critique lane compatible with the same request shape.
pub struct QianjiAdvisoryAuditExecutor {
    /// Orchestrator used to render per-role advisory prompt snapshots.
    pub orchestrator: Arc<ThousandFacesOrchestrator>,
    /// Persona registry used to resolve requested advisory roles.
    pub registry: Arc<PersonaRegistry>,
    /// Injection policy used to assemble typed advisory snapshots.
    pub injection_policy: InjectionPolicy,
    /// Fallback role used when the request does not specify any roles.
    pub default_role_id: String,
}

impl QianjiAdvisoryAuditExecutor {
    /// Create a new advisory executor bridge with default snapshot policy.
    #[must_use]
    pub fn new(
        orchestrator: Arc<ThousandFacesOrchestrator>,
        registry: Arc<PersonaRegistry>,
    ) -> Self {
        Self {
            orchestrator,
            registry,
            injection_policy: InjectionPolicy::default(),
            default_role_id: DEFAULT_ROLE_ID.to_string(),
        }
    }

    /// Override the injection policy used for advisory snapshot planning.
    #[must_use]
    pub fn with_injection_policy(mut self, injection_policy: InjectionPolicy) -> Self {
        self.injection_policy = injection_policy;
        self
    }

    /// Override the fallback role used when no explicit roles are requested.
    #[must_use]
    pub fn with_default_role_id(mut self, default_role_id: impl Into<String>) -> Self {
        self.default_role_id = default_role_id.into();
        self
    }

    /// Build a typed multi-role advisory execution plan.
    ///
    /// # Errors
    ///
    /// Returns an error when any requested role cannot be resolved from the persona registry, when
    /// the role snapshot cannot be assembled, or when the generated `InjectionSnapshot` violates
    /// the configured injection policy.
    pub async fn build_plan(
        &self,
        request: &AdvisoryAuditRequest,
    ) -> Result<QianjiAdvisoryExecutionPlan> {
        let resolved_roles = self.requested_roles(request);
        let role_mix = Self::build_role_mix(request, &resolved_roles);
        let session_id = Self::session_id(request);
        let primary_finding = primary_finding(&request.findings);
        let mut roles = Vec::with_capacity(resolved_roles.len());

        for (role_index, role_id) in resolved_roles.iter().enumerate() {
            let persona = self.resolve_persona(role_id)?;
            let blocks =
                Self::build_blocks(request, &session_id, &persona, primary_finding.as_ref());
            let narrative_blocks = blocks
                .iter()
                .map(|block| block.payload.clone())
                .collect::<Vec<_>>();
            let rendered_prompt = self
                .orchestrator
                .assemble_snapshot(&persona, narrative_blocks, "")
                .await
                .map_err(|error| {
                    anyhow!("failed to assemble advisory snapshot for '{role_id}': {error}")
                })?;
            let turn_id = u64::try_from(role_index + 1).map_err(|error| {
                anyhow!("role index overflow while preparing advisory plan: {error}")
            })?;
            let snapshot = InjectionSnapshot::from_blocks(
                snapshot_id(request, role_id),
                session_id.clone(),
                turn_id,
                self.injection_policy.clone(),
                Some(role_mix.clone()),
                blocks,
            );
            snapshot.validate().map_err(|error| {
                anyhow!("invalid advisory injection snapshot for role '{role_id}': {error}")
            })?;

            roles.push(QianjiAdvisoryRolePlan {
                role_id: role_id.clone(),
                persona_name: persona.name.clone(),
                snapshot,
                rendered_prompt,
            });
        }

        Ok(QianjiAdvisoryExecutionPlan {
            suite_id: request.suite_id.clone(),
            pack_id: request.pack_id.clone(),
            role_mix,
            roles,
        })
    }

    /// Build normalized scaffold findings for a previously prepared advisory plan.
    #[must_use]
    pub(crate) fn findings_from_plan(
        request: &AdvisoryAuditRequest,
        plan: &QianjiAdvisoryExecutionPlan,
    ) -> Vec<RoleAuditFinding> {
        let primary_finding = primary_finding(&request.findings);
        let trace_id = primary_trace_id(request);
        let runtime_trace_evidence = runtime_trace_evidence(request);

        plan.roles
            .iter()
            .map(|role_plan| {
                let mut finding = RoleAuditFinding::new(
                    role_plan.role_id.clone(),
                    primary_finding
                        .as_ref()
                        .map_or(xiuxian_testing::FindingSeverity::Warning, |finding| {
                            finding.severity
                        }),
                    advisory_summary(role_plan, request.findings.len(), primary_finding.as_ref()),
                );

                if let Some(finding_rule_id) = primary_finding
                    .as_ref()
                    .map(|finding| finding.rule_id.clone())
                {
                    finding.rule_id = Some(finding_rule_id);
                }
                if let Some(ref top_finding) = primary_finding {
                    finding.confidence = top_finding.confidence;
                    finding.why_it_matters = if top_finding.why_it_matters.trim().is_empty() {
                        top_finding.summary.clone()
                    } else {
                        top_finding.why_it_matters.clone()
                    };
                    finding.remediation = if top_finding.remediation.trim().is_empty() {
                        "Run the live formal audit critique lane and attach the streamed evidence."
                            .to_string()
                    } else {
                        top_finding.remediation.clone()
                    };
                    finding.examples = top_finding.examples.clone();
                    finding.evidence.extend(top_finding.evidence.clone());
                } else {
                    finding.why_it_matters =
                        "Prepared advisory review without upstream deterministic findings."
                            .to_string();
                    finding.remediation =
                        "Provide deterministic findings before invoking multi-role advisory review."
                            .to_string();
                }

                finding.trace_id.clone_from(&trace_id);
                finding.evidence.extend(runtime_trace_evidence.clone());
                finding.evidence.push(FindingEvidence {
                    kind: EvidenceKind::DerivedInvariant,
                    path: None,
                    locator: Some(role_plan.snapshot.snapshot_id.clone()),
                    message: format!(
                        "Prepared Qianhuan advisory snapshot for '{}' with {} blocks and {} chars.",
                        role_plan.persona_name,
                        role_plan.snapshot.blocks.len(),
                        role_plan.snapshot.total_chars
                    ),
                });
                finding.labels = advisory_labels(request, &plan.role_mix, role_plan);

                finding
            })
            .collect()
    }

    fn requested_roles(&self, request: &AdvisoryAuditRequest) -> Vec<String> {
        if request.requested_roles.is_empty() {
            return vec![self.default_role_id.clone()];
        }

        let mut roles = Vec::with_capacity(request.requested_roles.len());
        for role_id in &request.requested_roles {
            if !roles.contains(role_id) {
                roles.push(role_id.clone());
            }
        }
        roles
    }

    fn build_role_mix(request: &AdvisoryAuditRequest, roles: &[String]) -> RoleMixProfile {
        RoleMixProfile {
            profile_id: role_mix_profile_id(request),
            roles: roles
                .iter()
                .map(|role_id| RoleMixRole {
                    role: role_id.clone(),
                    weight: 1.0,
                })
                .collect(),
            rationale: format!(
                "Prepared advisory role mix for contract suite '{}' and pack '{}'.",
                request.suite_id, request.pack_id
            ),
        }
    }

    fn session_id(request: &AdvisoryAuditRequest) -> String {
        request
            .collection_context
            .labels
            .get("session_id")
            .cloned()
            .unwrap_or_else(|| format!("contract-audit:{}:{}", request.suite_id, request.pack_id))
    }

    fn resolve_persona(&self, role_id: &str) -> Result<PersonaProfile> {
        self.registry.get(role_id).ok_or_else(|| {
            anyhow!("advisory role '{role_id}' is not registered in PersonaRegistry")
        })
    }

    fn build_blocks(
        request: &AdvisoryAuditRequest,
        session_id: &str,
        persona: &PersonaProfile,
        primary_finding: Option<&ContractFinding>,
    ) -> Vec<PromptContextBlock> {
        let mut blocks = vec![PromptContextBlock::new(
            format!("{}:policy", sanitize_identifier(persona.id.as_str())),
            PromptContextSource::Policy,
            PromptContextCategory::Policy,
            1_000,
            session_id.to_string(),
            pack_summary(request),
            true,
        )];

        if !persona.style_anchors.is_empty() {
            blocks.push(PromptContextBlock::new(
                format!("{}:anchors", sanitize_identifier(persona.id.as_str())),
                PromptContextSource::Policy,
                PromptContextCategory::Policy,
                950,
                session_id.to_string(),
                format!(
                    "Role anchors for {}: {}",
                    persona.name,
                    persona.style_anchors.join(", ")
                ),
                true,
            ));
        }

        blocks.push(PromptContextBlock::new(
            format!("{}:findings", sanitize_identifier(persona.id.as_str())),
            PromptContextSource::RuntimeHint,
            PromptContextCategory::RuntimeHint,
            900,
            session_id.to_string(),
            findings_summary(&request.findings),
            false,
        ));

        if let Some(finding) = primary_finding {
            blocks.push(PromptContextBlock::new(
                format!("{}:primary", sanitize_identifier(persona.id.as_str())),
                PromptContextSource::Knowledge,
                PromptContextCategory::Knowledge,
                875,
                session_id.to_string(),
                primary_finding_summary(finding),
                false,
            ));
        }

        let runtime_trace_summary = runtime_trace_artifact_summary(request);
        if !runtime_trace_summary.is_empty() {
            blocks.push(PromptContextBlock::new(
                format!("{}:runtime", sanitize_identifier(persona.id.as_str())),
                PromptContextSource::RuntimeHint,
                PromptContextCategory::RuntimeHint,
                850,
                session_id.to_string(),
                runtime_trace_summary,
                false,
            ));
        }

        blocks
    }
}

#[async_trait]
impl AdvisoryAuditExecutor for QianjiAdvisoryAuditExecutor {
    async fn run(&self, request: AdvisoryAuditRequest) -> Result<Vec<RoleAuditFinding>> {
        let plan = self.build_plan(&request).await?;
        Ok(Self::findings_from_plan(&request, &plan))
    }
}

fn primary_finding(findings: &[ContractFinding]) -> Option<ContractFinding> {
    findings
        .iter()
        .cloned()
        .max_by_key(|finding| finding.severity)
}

fn primary_trace_id(request: &AdvisoryAuditRequest) -> Option<String> {
    request
        .findings
        .iter()
        .find_map(|finding| finding.trace_ids.first().cloned())
        .or_else(|| {
            request
                .artifacts
                .artifacts
                .iter()
                .find(|artifact| artifact.kind == ArtifactKind::RuntimeTrace)
                .and_then(|artifact| {
                    artifact
                        .labels
                        .get("trace_id")
                        .cloned()
                        .or_else(|| Some(artifact.id.clone()))
                })
        })
}

fn advisory_summary(
    role_plan: &QianjiAdvisoryRolePlan,
    finding_count: usize,
    primary_finding: Option<&ContractFinding>,
) -> String {
    let focus = primary_finding.map_or("contract review preparation", |finding| {
        finding.title.as_str()
    });
    format!(
        "{} prepared advisory review for {} deterministic finding(s); primary focus: {}.",
        role_plan.persona_name, finding_count, focus
    )
}

fn advisory_labels(
    request: &AdvisoryAuditRequest,
    role_mix: &RoleMixProfile,
    role_plan: &QianjiAdvisoryRolePlan,
) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert("source_lane".to_string(), "qianji_advisory".to_string());
    labels.insert("suite_id".to_string(), request.suite_id.clone());
    labels.insert("pack_id".to_string(), request.pack_id.clone());
    labels.insert("pack_version".to_string(), request.pack_version.clone());
    labels.insert("persona_name".to_string(), role_plan.persona_name.clone());
    labels.insert(
        "snapshot_id".to_string(),
        role_plan.snapshot.snapshot_id.clone(),
    );
    labels.insert(
        "role_mix_profile_id".to_string(),
        role_mix.profile_id.clone(),
    );
    labels.insert(
        "prompt_chars".to_string(),
        role_plan.rendered_prompt.chars().count().to_string(),
    );
    labels
}

fn pack_summary(request: &AdvisoryAuditRequest) -> String {
    let domains = if request.pack_domains.is_empty() {
        "none".to_string()
    } else {
        request.pack_domains.join(", ")
    };
    format!(
        "Contract suite: {}\nPack: {}@{}\nDomains: {}\nCrate: {}",
        request.suite_id,
        request.pack_id,
        request.pack_version,
        domains,
        request
            .collection_context
            .crate_name
            .as_deref()
            .unwrap_or("unknown")
    )
}

fn findings_summary(findings: &[ContractFinding]) -> String {
    if findings.is_empty() {
        return "No deterministic contract findings were provided.".to_string();
    }

    findings
        .iter()
        .map(|finding| {
            format!(
                "- [{:?}] {}: {}",
                finding.severity, finding.title, finding.summary
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn primary_finding_summary(finding: &ContractFinding) -> String {
    let why_it_matters = if finding.why_it_matters.trim().is_empty() {
        finding.summary.as_str()
    } else {
        finding.why_it_matters.as_str()
    };
    format!(
        "Primary contract focus: {}\nWhy it matters: {}\nSuggested remediation: {}",
        finding.title,
        why_it_matters,
        if finding.remediation.trim().is_empty() {
            "No remediation provided."
        } else {
            finding.remediation.as_str()
        }
    )
}

fn runtime_trace_artifact_summary(request: &AdvisoryAuditRequest) -> String {
    let runtime_artifacts = request
        .artifacts
        .artifacts
        .iter()
        .filter(|artifact| artifact.kind == ArtifactKind::RuntimeTrace)
        .collect::<Vec<_>>();

    if runtime_artifacts.is_empty() {
        return String::new();
    }

    runtime_artifacts
        .iter()
        .map(|artifact| {
            let trace_id = artifact
                .labels
                .get("trace_id")
                .map_or(artifact.id.as_str(), String::as_str);
            format!("Runtime trace available: {trace_id}")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn runtime_trace_evidence(request: &AdvisoryAuditRequest) -> Vec<FindingEvidence> {
    request
        .artifacts
        .artifacts
        .iter()
        .filter(|artifact| artifact.kind == ArtifactKind::RuntimeTrace)
        .map(|artifact| FindingEvidence {
            kind: EvidenceKind::RuntimeTrace,
            path: artifact.path.clone(),
            locator: Some(artifact.id.clone()),
            message: artifact.labels.get("trace_id").map_or_else(
                || {
                    format!(
                        "Runtime trace artifact '{}' is available for advisory review.",
                        artifact.id
                    )
                },
                |trace_id| format!("Runtime trace available for advisory review: {trace_id}"),
            ),
        })
        .collect()
}

fn role_mix_profile_id(request: &AdvisoryAuditRequest) -> String {
    format!(
        "contract-audit:{}:{}",
        sanitize_identifier(request.suite_id.as_str()),
        sanitize_identifier(request.pack_id.as_str())
    )
}

fn snapshot_id(request: &AdvisoryAuditRequest, role_id: &str) -> String {
    format!(
        "{}:{}:{}",
        role_mix_profile_id(request),
        sanitize_identifier(role_id),
        "snapshot"
    )
}

fn sanitize_identifier(raw: &str) -> String {
    let mut sanitized = String::with_capacity(raw.len());
    for character in raw.chars() {
        if character.is_ascii_alphanumeric() {
            sanitized.push(character.to_ascii_lowercase());
        } else {
            sanitized.push('-');
        }
    }
    sanitized
}
