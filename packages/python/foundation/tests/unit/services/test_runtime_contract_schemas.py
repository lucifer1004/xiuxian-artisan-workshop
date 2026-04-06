"""Validation tests for newly introduced runtime contract schemas."""

from __future__ import annotations

import json

from jsonschema import Draft202012Validator

from xiuxian_foundation.api.schema_locator import resolve_schema_file_path


def _load_schema(name: str) -> dict:
    path = resolve_schema_file_path(name)
    assert path.exists(), f"schema missing: {path}"
    return json.loads(path.read_text(encoding="utf-8"))


def _validate(schema_name: str, payload: dict) -> None:
    schema = _load_schema(schema_name)
    validator = Draft202012Validator(schema)
    errors = list(validator.iter_errors(payload))
    assert not errors, "; ".join(error.message for error in errors)


def test_discover_match_schema_accepts_contract_payload() -> None:
    payload = {
        "tool": "skill.discover",
        "usage": 'tool: skill.discover with {"intent": "<intent: string>"}',
        "score": 0.67,
        "final_score": 0.82,
        "confidence": "high",
        "ranking_reason": "vector=0.91 | keyword=0.11 | final=0.82",
        "input_schema_digest": "sha256:abc123def456",
        "documentation_path": "/tmp/SKILL.md",
    }
    _validate("xiuxian.discover.match.v1.schema.json", payload)


def test_memory_gate_event_schema_accepts_contract_payload() -> None:
    payload = {
        "session_id": "telegram:group-1:user-9",
        "turn_id": 42,
        "memory_id": "mem:9c2",
        "state_before": "active",
        "state_after": "promoted",
        "ttl_score": 0.91,
        "decision": {
            "verdict": "promote",
            "confidence": 0.89,
            "react_evidence_refs": ["react:fix_retry:12"],
            "graph_evidence_refs": ["graph:path:resolve->verify"],
            "omega_factors": ["utility_trend=up"],
            "reason": "High utility and repeated revalidation success",
            "next_action": "promote",
            "promotion_target": "working_knowledge",
        },
    }
    _validate("xiuxian.memory.gate_event.v1.schema.json", payload)


def test_route_trace_schema_accepts_contract_payload() -> None:
    payload = {
        "session_id": "telegram:group-1:user-9",
        "turn_id": 43,
        "selected_route": "graph",
        "confidence": 0.84,
        "risk_level": "medium",
        "tool_trust_class": "evidence",
        "fallback_applied": False,
        "fallback_policy": "retry_react",
        "tool_chain": ["skill.discover", "knowledge.search"],
        "latency_ms": 327.1,
        "failure_taxonomy": [],
        "injection": {
            "blocks_used": 6,
            "chars_injected": 3120,
            "dropped_by_budget": 1,
        },
    }
    _validate("xiuxian.runtime.route_trace.v1.schema.json", payload)


def test_route_trace_schema_accepts_graph_step_aggregation_payload() -> None:
    payload = {
        "session_id": "telegram:group-1:user-9",
        "turn_id": 44,
        "selected_route": "graph",
        "confidence": 0.92,
        "risk_level": "low",
        "tool_trust_class": "verification",
        "fallback_applied": True,
        "fallback_policy": "switch_to_graph",
        "tool_chain": ["bridge.flaky"],
        "latency_ms": 141.7,
        "failure_taxonomy": ["transport"],
        "plan_id": "graph-plan:omega:bridge.flaky:switch_to_graph:verification",
        "workflow_mode": "omega",
        "graph_steps": [
            {
                "index": 1,
                "id": "prepare_injection_context",
                "kind": "prepare_injection_context",
                "attempt": 0,
                "latency_ms": 0.4,
                "status": "prepared",
            },
            {
                "index": 2,
                "id": "invoke_graph_tool",
                "kind": "invoke_graph_tool",
                "attempt": 1,
                "latency_ms": 70.2,
                "status": "tool_call_transport_failed",
                "failure_reason": "connection refused",
                "tool_name": "bridge.flaky",
            },
            {
                "index": 3,
                "id": "evaluate_fallback",
                "kind": "evaluate_fallback",
                "attempt": 2,
                "latency_ms": 40.8,
                "status": "retry_succeeded_without_metadata",
                "fallback_action": "retry_bridge_without_metadata",
            },
        ],
    }
    _validate("xiuxian.runtime.route_trace.v1.schema.json", payload)
