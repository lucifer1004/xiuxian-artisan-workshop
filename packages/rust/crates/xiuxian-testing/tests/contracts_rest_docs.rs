//! Focused integration coverage for the built-in `rest_docs` rule pack.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::json;
use xiuxian_testing::{
    ArtifactKind, CollectedArtifact, CollectedArtifacts, FindingSeverity, RestDocsRulePack,
    RulePack,
};

fn openapi_artifact(content: serde_json::Value) -> CollectedArtifact {
    CollectedArtifact {
        id: "wendao-openapi".to_string(),
        kind: ArtifactKind::OpenApiDocument,
        path: Some(PathBuf::from("tests/fixtures/openapi.json")),
        content,
        labels: BTreeMap::new(),
    }
}

#[test]
fn rest_docs_pack_flags_missing_purpose_and_response_descriptions() {
    let artifacts = CollectedArtifacts {
        artifacts: vec![openapi_artifact(json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Gateway",
                "version": "v1"
            },
            "paths": {
                "/health": {
                    "get": {
                        "responses": {
                            "200": {
                                "description": ""
                            },
                            "500": {
                                "content": {
                                    "application/json": {
                                        "schema": {
                                            "type": "object"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }))],
        metadata: BTreeMap::new(),
    };

    let findings_result = RestDocsRulePack.evaluate(&artifacts);
    assert!(findings_result.is_ok());
    let findings = findings_result.unwrap_or_default();

    assert_eq!(findings.len(), 2);

    let purpose = findings
        .iter()
        .find(|finding| finding.rule_id == "REST-R001");
    assert!(purpose.is_some());
    let purpose = purpose.unwrap_or_else(|| unreachable!("checked above"));
    assert_eq!(purpose.severity, FindingSeverity::Error);
    assert_eq!(
        purpose.labels.get("path").map(String::as_str),
        Some("/health")
    );
    assert_eq!(
        purpose
            .evidence
            .first()
            .and_then(|evidence| evidence.locator.as_deref()),
        Some("/paths/~1health/get")
    );

    let responses = findings
        .iter()
        .find(|finding| finding.rule_id == "REST-R003");
    assert!(responses.is_some());
    let responses = responses.unwrap_or_else(|| unreachable!("checked above"));
    assert_eq!(responses.severity, FindingSeverity::Error);
    assert_eq!(responses.evidence.len(), 2);
    assert!(
        responses
            .evidence
            .iter()
            .any(|evidence| evidence.message.contains("Success response `200`"))
    );
    assert!(
        responses
            .evidence
            .iter()
            .any(|evidence| evidence.message.contains("Error response `500`"))
    );
}

#[test]
fn rest_docs_pack_flags_missing_examples_for_non_trivial_request_body() {
    let artifacts = CollectedArtifacts {
        artifacts: vec![openapi_artifact(json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Gateway",
                "version": "v1"
            },
            "paths": {
                "/documents": {
                    "post": {
                        "summary": "Create a document",
                        "responses": {
                            "201": {
                                "description": "Document created."
                            },
                            "400": {
                                "description": "Invalid request."
                            }
                        },
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "title": {
                                                "type": "string"
                                            },
                                            "body": {
                                                "type": "string"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }))],
        metadata: BTreeMap::new(),
    };

    let findings_result = RestDocsRulePack.evaluate(&artifacts);
    assert!(findings_result.is_ok());
    let findings = findings_result.unwrap_or_default();

    assert_eq!(findings.len(), 1);
    let finding = &findings[0];
    assert_eq!(finding.rule_id, "REST-R007");
    assert_eq!(finding.severity, FindingSeverity::Warning);
    assert_eq!(
        finding
            .evidence
            .first()
            .and_then(|evidence| evidence.locator.as_deref()),
        Some("/paths/~1documents/post/requestBody")
    );
    assert!(
        finding
            .evidence
            .first()
            .is_some_and(|evidence| evidence.message.contains("application/json"))
    );
}

#[test]
fn rest_docs_pack_accepts_well_documented_ref_backed_operation() {
    let artifacts = CollectedArtifacts {
        artifacts: vec![openapi_artifact(json!({
            "openapi": "3.1.0",
            "info": {
                "title": "Gateway",
                "version": "v1"
            },
            "components": {
                "schemas": {
                    "CreateDocumentRequest": {
                        "type": "object",
                        "properties": {
                            "title": {
                                "type": "string"
                            },
                            "body": {
                                "type": "string"
                            }
                        },
                        "example": {
                            "title": "Roadmap",
                            "body": "Ship contract testing."
                        }
                    }
                }
            },
            "paths": {
                "/documents": {
                    "post": {
                        "summary": "Create a document",
                        "description": "Create a persisted document resource.",
                        "responses": {
                            "201": {
                                "description": "Document created."
                            },
                            "400": {
                                "description": "Invalid request."
                            }
                        },
                        "requestBody": {
                            "required": true,
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": "#/components/schemas/CreateDocumentRequest"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }))],
        metadata: BTreeMap::new(),
    };

    let findings_result = RestDocsRulePack.evaluate(&artifacts);
    assert!(findings_result.is_ok());
    let findings = findings_result.unwrap_or_default();

    assert!(findings.is_empty());
}
