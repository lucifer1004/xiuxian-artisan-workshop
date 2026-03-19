//! Deterministic REST documentation checks over normalized `OpenAPI` artifacts.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use serde_json::{Map, Value};

use super::super::model::{
    ArtifactKind, CollectedArtifact, CollectedArtifacts, CollectionContext, ContractFinding,
    EvidenceKind, FindingEvidence, FindingMode, FindingSeverity,
};
use super::super::rule_pack::{RulePack, RulePackDescriptor};

const HTTP_METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];
const PACK_ID: &str = "rest_docs";

/// Deterministic V1 REST contract checks over normalized `OpenAPI` documents.
#[derive(Debug, Default, Clone, Copy)]
pub struct RestDocsRulePack;

impl RulePack for RestDocsRulePack {
    fn descriptor(&self) -> RulePackDescriptor {
        RulePackDescriptor {
            id: PACK_ID,
            version: "v1",
            domains: &["rest", "docs", "openapi"],
            default_mode: FindingMode::Deterministic,
        }
    }

    fn collect(&self, _ctx: &CollectionContext) -> Result<CollectedArtifacts> {
        Ok(CollectedArtifacts::default())
    }

    fn evaluate(&self, artifacts: &CollectedArtifacts) -> Result<Vec<ContractFinding>> {
        let mut findings = Vec::new();

        for artifact in &artifacts.artifacts {
            if artifact.kind != ArtifactKind::OpenApiDocument {
                continue;
            }

            findings.extend(RestDocsEvaluator::new(artifact).evaluate());
        }

        Ok(findings)
    }
}

struct RestDocsEvaluator<'a> {
    artifact: &'a CollectedArtifact,
}

impl<'a> RestDocsEvaluator<'a> {
    fn new(artifact: &'a CollectedArtifact) -> Self {
        Self { artifact }
    }

    fn evaluate(&self) -> Vec<ContractFinding> {
        let mut findings = Vec::new();
        let Some(paths) = self
            .artifact
            .content
            .get("paths")
            .and_then(Value::as_object)
        else {
            return findings;
        };

        for (path_name, path_item) in paths {
            let Some(path_object) = self.resolve_object(path_item) else {
                continue;
            };

            for method in HTTP_METHODS {
                let Some(operation_value) = path_object.get(method) else {
                    continue;
                };
                let Some(operation) = self.resolve_object(operation_value) else {
                    continue;
                };

                findings.extend(self.check_endpoint_purpose(path_name, method, operation));
                findings.extend(self.check_response_documentation(path_name, method, operation));
                findings.extend(self.check_request_examples(path_name, method, operation));
            }
        }

        findings
    }

    fn check_endpoint_purpose(
        &self,
        path_name: &str,
        method: &str,
        operation: &Map<String, Value>,
    ) -> Option<ContractFinding> {
        let summary = operation.get("summary").and_then(Value::as_str);
        let description = operation.get("description").and_then(Value::as_str);
        if !is_blank(summary) || !is_blank(description) {
            return None;
        }

        let mut finding = self.base_finding(
            "REST-R001",
            FindingSeverity::Error,
            path_name,
            method,
            "Missing endpoint purpose",
            format!(
                "The {} {} operation is missing both `summary` and `description`.",
                method.to_uppercase(),
                path_name
            ),
        );
        finding.why_it_matters = "External callers, reviewers, and knowledge-indexing pipelines need a stable purpose statement for every reachable endpoint.".to_string();
        finding.remediation = "Add a non-empty `summary` or `description` that explains what the endpoint does and when callers should use it.".to_string();
        finding
            .examples
            .good
            .push("GET /health includes a short summary like `Check gateway health`.".to_string());
        finding.examples.bad.push(
            "GET /health exposes only response schemas with no human-readable purpose.".to_string(),
        );
        finding.evidence.push(self.open_api_evidence(
            path_name,
            method,
            None,
            "Operation is missing both `summary` and `description`.".to_string(),
        ));
        Some(finding)
    }

    fn check_response_documentation(
        &self,
        path_name: &str,
        method: &str,
        operation: &Map<String, Value>,
    ) -> Option<ContractFinding> {
        let Some(responses) = operation.get("responses").and_then(Value::as_object) else {
            return Some(self.missing_responses_finding(path_name, method));
        };

        let issues = self.collect_response_issues(responses);
        if issues.is_empty() {
            return None;
        }

        Some(self.response_documentation_finding(path_name, method, issues))
    }

    fn check_request_examples(
        &self,
        path_name: &str,
        method: &str,
        operation: &Map<String, Value>,
    ) -> Option<ContractFinding> {
        let request_body = operation.get("requestBody")?;
        let request_body_object = self.resolve_object(request_body)?;
        let content = request_body_object
            .get("content")
            .and_then(Value::as_object)?;

        let mut requires_example = false;
        let mut documented_examples = false;
        let mut missing_media_types = Vec::new();

        for (media_type, media_value) in content {
            let Some(media_object) = media_value.as_object() else {
                continue;
            };
            let schema = media_object.get("schema");
            if !self.schema_is_non_trivial(schema) {
                continue;
            }

            requires_example = true;
            if media_type_has_examples(media_object) || self.schema_has_examples(schema) {
                documented_examples = true;
                continue;
            }

            missing_media_types.push(media_type.clone());
        }

        if !requires_example || documented_examples {
            return None;
        }

        let mut finding = self.base_finding(
            "REST-R007",
            FindingSeverity::Warning,
            path_name,
            method,
            "Missing request-body example",
            format!(
                "The {} {} operation has a non-trivial request body but no request example.",
                method.to_uppercase(),
                path_name
            ),
        );
        finding.why_it_matters = "Concrete request examples make REST contracts easier to review, test, and consume correctly, especially when the schema is object-shaped or referenced.".to_string();
        finding.remediation = "Add `example` or `examples` data to at least one non-trivial request media type or its resolved schema.".to_string();
        finding.examples.good.push(
            "Provide an `application/json` example that mirrors a realistic request payload."
                .to_string(),
        );
        finding.examples.bad.push(
            "Define an object schema with several fields but leave all request examples empty."
                .to_string(),
        );
        finding.evidence.push(self.open_api_evidence(
            path_name,
            method,
            Some("/requestBody"),
            format!(
                "Non-trivial request media types are missing examples: {}.",
                missing_media_types.join(", ")
            ),
        ));
        Some(finding)
    }

    fn missing_responses_finding(&self, path_name: &str, method: &str) -> ContractFinding {
        let mut finding = self.base_finding(
            "REST-R003",
            FindingSeverity::Error,
            path_name,
            method,
            "Missing response documentation",
            format!(
                "The {} {} operation does not declare any documented responses.",
                method.to_uppercase(),
                path_name
            ),
        );
        finding.why_it_matters = "REST contracts need explicit success and error response coverage so clients and reviewers can reason about expected behavior.".to_string();
        finding.remediation =
            "Add documented success and error responses with non-empty descriptions.".to_string();
        finding.evidence.push(self.open_api_evidence(
            path_name,
            method,
            Some("/responses"),
            "Operation is missing the `responses` object.".to_string(),
        ));
        finding
    }

    fn collect_response_issues(&self, responses: &Map<String, Value>) -> Vec<ResponseIssue> {
        let mut issues = Vec::new();
        self.extend_response_issues_for_class(responses, ResponseClass::Success, &mut issues);
        self.extend_response_issues_for_class(responses, ResponseClass::Error, &mut issues);
        issues
    }

    fn extend_response_issues_for_class(
        &self,
        responses: &Map<String, Value>,
        class: ResponseClass,
        issues: &mut Vec<ResponseIssue>,
    ) {
        let statuses = collect_statuses(responses, |status| class.matches_status(status));
        if statuses.is_empty() {
            issues.push(ResponseIssue {
                locator_suffix: "/responses".to_string(),
                message: class.missing_coverage_message().to_string(),
            });
            return;
        }

        for status in statuses {
            let Some(response) = responses
                .get(status)
                .and_then(|value| self.resolve_object(value))
            else {
                continue;
            };
            if has_non_empty_description(response) {
                continue;
            }

            issues.push(ResponseIssue {
                locator_suffix: format!("/responses/{status}"),
                message: class.missing_description_message(status),
            });
        }
    }

    fn response_documentation_finding(
        &self,
        path_name: &str,
        method: &str,
        issues: Vec<ResponseIssue>,
    ) -> ContractFinding {
        let mut finding = self.base_finding(
            "REST-R003",
            FindingSeverity::Error,
            path_name,
            method,
            "Incomplete response documentation",
            format!(
                "The {} {} operation is missing required response documentation.",
                method.to_uppercase(),
                path_name
            ),
        );
        finding.why_it_matters = "Clients need documented success and error responses to handle the API safely and to keep generated contracts aligned with implementation intent.".to_string();
        finding.remediation = "Document at least one success response and one error response, and give each response a non-empty description.".to_string();
        finding.examples.good.push("Document `200` and `400` responses with short descriptions such as `Query succeeded` and `Invalid request`.".to_string());
        finding.examples.bad.push(
            "Expose response status codes without descriptions or omit error responses entirely."
                .to_string(),
        );
        finding.evidence.extend(issues.into_iter().map(|issue| {
            self.open_api_evidence(
                path_name,
                method,
                Some(&issue.locator_suffix),
                issue.message,
            )
        }));
        finding
    }

    fn base_finding(
        &self,
        rule_id: &str,
        severity: FindingSeverity,
        path_name: &str,
        method: &str,
        title: &str,
        summary: String,
    ) -> ContractFinding {
        let mut finding = ContractFinding::new(
            rule_id,
            PACK_ID,
            severity,
            FindingMode::Deterministic,
            title,
            summary,
        );
        finding.labels = self.labels_for_operation(path_name, method);
        finding
    }

    fn labels_for_operation(&self, path_name: &str, method: &str) -> BTreeMap<String, String> {
        let mut labels = BTreeMap::new();
        labels.insert("artifact_id".to_string(), self.artifact.id.clone());
        labels.insert("http_method".to_string(), method.to_uppercase());
        labels.insert("path".to_string(), path_name.to_string());
        labels
    }

    fn open_api_evidence(
        &self,
        path_name: &str,
        method: &str,
        locator_suffix: Option<&str>,
        message: String,
    ) -> FindingEvidence {
        let mut locator = format!("/paths/{}/{}", escape_json_pointer(path_name), method);
        if let Some(suffix) = locator_suffix {
            locator.push_str(suffix);
        }

        FindingEvidence {
            kind: EvidenceKind::OpenApiNode,
            path: self.artifact.path.clone(),
            locator: Some(locator),
            message,
        }
    }

    fn resolve_object<'b>(&'b self, value: &'b Value) -> Option<&'b Map<String, Value>> {
        resolve_value(&self.artifact.content, value, 0)?.as_object()
    }

    fn schema_is_non_trivial(&self, schema: Option<&Value>) -> bool {
        let Some(resolved) =
            schema.and_then(|value| resolve_value(&self.artifact.content, value, 0))
        else {
            return false;
        };
        let Some(schema_object) = resolved.as_object() else {
            return false;
        };

        if schema_object.contains_key("properties")
            || schema_object.contains_key("items")
            || schema_object.contains_key("allOf")
            || schema_object.contains_key("anyOf")
            || schema_object.contains_key("oneOf")
        {
            return true;
        }

        matches!(
            schema_object.get("type").and_then(Value::as_str),
            Some("object" | "array")
        )
    }

    fn schema_has_examples(&self, schema: Option<&Value>) -> bool {
        let Some(resolved) =
            schema.and_then(|value| resolve_value(&self.artifact.content, value, 0))
        else {
            return false;
        };
        let Some(schema_object) = resolved.as_object() else {
            return false;
        };

        if schema_object.contains_key("example") {
            return true;
        }

        schema_object
            .get("examples")
            .and_then(Value::as_array)
            .is_some_and(|examples| !examples.is_empty())
    }
}

struct ResponseIssue {
    locator_suffix: String,
    message: String,
}

#[derive(Debug, Clone, Copy)]
enum ResponseClass {
    Success,
    Error,
}

impl ResponseClass {
    fn matches_status(self, status: &str) -> bool {
        match self {
            Self::Success => is_success_status(status),
            Self::Error => is_error_status(status),
        }
    }

    fn missing_coverage_message(self) -> &'static str {
        match self {
            Self::Success => "Operation is missing a documented success response.",
            Self::Error => "Operation is missing a documented error response.",
        }
    }

    fn missing_description_message(self, status: &str) -> String {
        match self {
            Self::Success => {
                format!("Success response `{status}` is missing a non-empty description.")
            }
            Self::Error => format!("Error response `{status}` is missing a non-empty description."),
        }
    }
}

fn collect_statuses<P>(responses: &Map<String, Value>, predicate: P) -> BTreeSet<&str>
where
    P: Fn(&str) -> bool,
{
    responses
        .keys()
        .filter(|status| predicate(status.as_str()))
        .map(String::as_str)
        .collect()
}

fn is_success_status(status: &str) -> bool {
    status.starts_with('2')
}

fn is_error_status(status: &str) -> bool {
    status == "default" || status.starts_with('4') || status.starts_with('5')
}

fn has_non_empty_description(object: &Map<String, Value>) -> bool {
    !is_blank(object.get("description").and_then(Value::as_str))
}

fn media_type_has_examples(media_type: &Map<String, Value>) -> bool {
    if media_type.contains_key("example") {
        return true;
    }

    media_type
        .get("examples")
        .and_then(Value::as_object)
        .is_some_and(|examples| !examples.is_empty())
}

fn resolve_value<'a>(document: &'a Value, value: &'a Value, depth: usize) -> Option<&'a Value> {
    if depth > 8 {
        return Some(value);
    }

    let Some(reference) = value.get("$ref").and_then(Value::as_str) else {
        return Some(value);
    };
    let pointer = reference.strip_prefix('#')?;
    document
        .pointer(pointer)
        .and_then(|resolved| resolve_value(document, resolved, depth + 1))
        .or(Some(value))
}

fn escape_json_pointer(segment: &str) -> String {
    segment.replace('~', "~0").replace('/', "~1")
}

fn is_blank(value: Option<&str>) -> bool {
    value.is_none_or(|text| text.trim().is_empty())
}
