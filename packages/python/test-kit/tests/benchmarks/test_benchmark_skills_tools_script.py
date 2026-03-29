"""Unit tests for scripts/benchmark_skills_tools.py case resolution."""

from __future__ import annotations

from functools import lru_cache
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
from typing import TYPE_CHECKING
from urllib.parse import unquote, urlparse

from xiuxian_wendao_py.compat.runtime import get_project_root

if TYPE_CHECKING:
    from types import ModuleType


@lru_cache(maxsize=1)
def _load_benchmark_script_module() -> ModuleType:
    """Load benchmark_skills_tools.py as a module for unit testing."""
    script_path = get_project_root() / "scripts" / "benchmark_skills_tools.py"
    spec = spec_from_file_location("benchmark_skills_tools_script", script_path)
    assert spec is not None
    assert spec.loader is not None
    module = module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_resolve_cases_defaults_include_local_and_network_crawl4ai() -> None:
    """Default case resolution should include both crawl4ai scenarios."""
    module = _load_benchmark_script_module()
    cases = module._resolve_cases(
        None,
        crawl4ai_scenarios="both",
        include_embedding_tools=False,
        include_cli_runner_cases=True,
    )
    case_ids = {str(case["case_id"]) for case in cases}

    assert module.CRAWL4AI_TOOL in case_ids
    assert f"{module.CRAWL4AI_TOOL}.{module.CRAWL4AI_SCENARIO_LOCAL}" in case_ids


def test_resolve_cases_network_only_for_crawl4ai() -> None:
    """Network-only mode should emit only the network crawl4ai case."""
    module = _load_benchmark_script_module()
    cases = module._resolve_cases(
        [module.CRAWL4AI_TOOL],
        crawl4ai_scenarios="network",
        include_embedding_tools=False,
        include_cli_runner_cases=True,
    )
    case_ids = {str(case["case_id"]) for case in cases}

    assert case_ids == {module.CRAWL4AI_TOOL}


def test_resolve_cases_expands_crawl4ai_tool_to_dual_cases_in_both_mode() -> None:
    """Explicit crawl4ai selection should include local + network in both mode."""
    module = _load_benchmark_script_module()
    cases = module._resolve_cases(
        [module.CRAWL4AI_TOOL],
        crawl4ai_scenarios="both",
        include_embedding_tools=False,
        include_cli_runner_cases=True,
    )
    case_ids = {str(case["case_id"]) for case in cases}

    assert module.CRAWL4AI_TOOL in case_ids
    assert f"{module.CRAWL4AI_TOOL}.{module.CRAWL4AI_SCENARIO_LOCAL}" in case_ids


def test_crawl4ai_local_fixture_case_uses_existing_file_url() -> None:
    """Local crawl4ai case should point to a deterministic fixture file URL."""
    module = _load_benchmark_script_module()
    catalog = module._build_case_catalog(
        crawl4ai_scenarios="local",
        include_embedding_tools=False,
        include_cli_runner_cases=False,
    )
    local_key = f"{module.CRAWL4AI_TOOL}.{module.CRAWL4AI_SCENARIO_LOCAL}"
    local_case = catalog[local_key]
    local_url = str(local_case["args"]["url"])

    assert local_url.startswith("file://")
    parsed = urlparse(local_url)
    fixture_path = Path(unquote(parsed.path))
    assert fixture_path.exists()


def test_omnicell_case_uses_observe_intent_and_fixture_scope() -> None:
    """omniCell benchmark should use deterministic observe-only fixture command."""
    module = _load_benchmark_script_module()
    cases = module._resolve_cases(
        ["omniCell.nuShell"],
        crawl4ai_scenarios="both",
        include_embedding_tools=False,
        include_cli_runner_cases=True,
    )
    assert len(cases) == 1
    case = cases[0]
    args = case["args"]

    assert args["intent"] == "observe"
    assert "scripts/fixtures/benchmark" in args["command"]


def test_embedding_tools_excluded_by_default() -> None:
    """Embedding-dependent tools should be excluded from default deterministic catalog."""
    module = _load_benchmark_script_module()
    catalog = module._build_case_catalog(
        crawl4ai_scenarios="local",
        include_embedding_tools=False,
        include_cli_runner_cases=False,
    )

    assert "memory.search_memory" not in catalog


def test_embedding_tools_included_when_enabled() -> None:
    """Embedding-dependent tools should be available when explicitly enabled."""
    module = _load_benchmark_script_module()
    catalog = module._build_case_catalog(
        crawl4ai_scenarios="local",
        include_embedding_tools=True,
        include_cli_runner_cases=False,
    )

    assert "memory.search_memory" in catalog


def test_resolve_cases_defaults_include_cli_runner_modes() -> None:
    """Default resolution should include CLI runner cold/warm/no-reuse cases."""
    module = _load_benchmark_script_module()
    cases = module._resolve_cases(
        None,
        crawl4ai_scenarios="local",
        include_embedding_tools=False,
        include_cli_runner_cases=True,
    )
    case_ids = {str(case["case_id"]) for case in cases}

    for case_id in module.CLI_RUNNER_CASE_IDS:
        assert case_id in case_ids
    for case_id in module.CLI_RUNNER_KNOWLEDGE_CASE_IDS:
        assert case_id in case_ids


def test_resolve_cases_cli_runner_tool_expands_to_all_modes() -> None:
    """Selecting `cli.skill_run` should expand to all CLI runner benchmark cases."""
    module = _load_benchmark_script_module()
    cases = module._resolve_cases(
        [module.CLI_RUNNER_TOOL],
        crawl4ai_scenarios="local",
        include_embedding_tools=False,
        include_cli_runner_cases=True,
    )
    case_ids = {str(case["case_id"]) for case in cases}
    assert case_ids == set(module.CLI_RUNNER_CASE_IDS) | set(module.CLI_RUNNER_KNOWLEDGE_CASE_IDS)


def test_resolve_cases_cli_runner_knowledge_alias_expands_profile_modes() -> None:
    """Selecting `cli.skill_run.knowledge_search` should expand its three mode cases."""
    module = _load_benchmark_script_module()
    cases = module._resolve_cases(
        [f"{module.CLI_RUNNER_TOOL}.{module.CLI_RUNNER_KNOWLEDGE_PROFILE}"],
        crawl4ai_scenarios="local",
        include_embedding_tools=False,
        include_cli_runner_cases=True,
    )
    case_ids = {str(case["case_id"]) for case in cases}
    assert case_ids == set(module.CLI_RUNNER_KNOWLEDGE_CASE_IDS)


def test_resolve_cases_can_disable_cli_runner_cases() -> None:
    """CLI runner cases should be omitted when explicitly disabled."""
    module = _load_benchmark_script_module()
    cases = module._resolve_cases(
        None,
        crawl4ai_scenarios="local",
        include_embedding_tools=False,
        include_cli_runner_cases=False,
    )
    case_ids = {str(case["case_id"]) for case in cases}
    assert all(not case_id.startswith(f"{module.CLI_RUNNER_TOOL}.") for case_id in case_ids)


def test_prepare_results_for_snapshot_gate_filters_network_in_deterministic_scope() -> None:
    """Deterministic gate scope should skip noisy crawl4ai network scenario."""
    module = _load_benchmark_script_module()
    results = [
        {
            "tool": module.CRAWL4AI_TOOL,
            "scenario": module.CRAWL4AI_SCENARIO_NETWORK,
            "avg_ms": 120.0,
            "ok": True,
        },
        {"tool": "knowledge.recall", "avg_ms": 28.0, "ok": True},
    ]

    filtered = module._prepare_results_for_snapshot_gate(
        results,
        gate_scope=module.SNAPSHOT_GATE_SCOPE_DETERMINISTIC,
        default_metric=module.SNAPSHOT_METRIC_P50,
        network_metric=module.SNAPSHOT_NETWORK_METRIC_TRIMMED_AVG,
    )

    assert len(filtered) == 1
    assert filtered[0]["tool"] == "knowledge.recall"


def test_prepare_results_for_snapshot_gate_uses_trimmed_metric_for_network() -> None:
    """Network case should expose trimmed metric override when configured."""
    module = _load_benchmark_script_module()
    results = [
        {
            "tool": module.CRAWL4AI_TOOL,
            "scenario": module.CRAWL4AI_SCENARIO_NETWORK,
            "avg_ms": 200.0,
            "p95_ms": 900.0,
            "trimmed_avg_ms": 260.0,
            "ok": True,
        }
    ]

    prepared = module._prepare_results_for_snapshot_gate(
        results,
        gate_scope=module.SNAPSHOT_GATE_SCOPE_ALL,
        default_metric=module.SNAPSHOT_METRIC_P50,
        network_metric=module.SNAPSHOT_NETWORK_METRIC_TRIMMED_AVG,
    )

    assert len(prepared) == 1
    assert prepared[0]["anomaly_observed_ms"] == 260.0
    assert prepared[0]["anomaly_observed_metric"] == "trimmed_avg_ms"


def test_build_cli_runner_summary_groups_profiles_and_comparisons() -> None:
    """CLI runner summary should group demo/knowledge profiles and expose p50 comparisons."""
    module = _load_benchmark_script_module()
    results = [
        {
            "tool": "cli.skill_run.default_cold",
            "tool_name": "cli.skill_run",
            "p50_ms": 900.0,
            "p95_ms": 980.0,
            "avg_ms": 910.0,
            "ok": True,
        },
        {
            "tool": "cli.skill_run.default_warm",
            "tool_name": "cli.skill_run",
            "p50_ms": 300.0,
            "p95_ms": 340.0,
            "avg_ms": 310.0,
            "ok": True,
        },
        {
            "tool": "cli.skill_run.no_reuse",
            "tool_name": "cli.skill_run",
            "p50_ms": 500.0,
            "p95_ms": 520.0,
            "avg_ms": 505.0,
            "ok": True,
        },
        {
            "tool": "cli.skill_run.knowledge_search.default_cold",
            "tool_name": "cli.skill_run",
            "p50_ms": 1100.0,
            "p95_ms": 1200.0,
            "avg_ms": 1120.0,
            "ok": True,
        },
        {
            "tool": "cli.skill_run.knowledge_search.default_warm",
            "tool_name": "cli.skill_run",
            "p50_ms": 420.0,
            "p95_ms": 450.0,
            "avg_ms": 425.0,
            "ok": True,
        },
        {
            "tool": "cli.skill_run.knowledge_search.no_reuse",
            "tool_name": "cli.skill_run",
            "p50_ms": 700.0,
            "p95_ms": 760.0,
            "avg_ms": 710.0,
            "ok": True,
        },
        {
            "tool": "knowledge.search",
            "tool_name": "knowledge.search",
            "p50_ms": 7.0,
            "p95_ms": 8.0,
            "avg_ms": 7.2,
            "ok": True,
        },
    ]
    summary = module._build_cli_runner_summary(results)

    assert summary["case_count"] == 6
    profiles = summary["profiles"]
    assert set(profiles.keys()) == {"demo_hello", "knowledge_search"}

    demo_cmp = profiles["demo_hello"]["comparisons"]
    assert demo_cmp["warm_vs_no_reuse_p50_delta_ms"] == -200.0
    assert demo_cmp["warm_vs_default_cold_p50_ratio"] == 0.33

    kg_cmp = profiles["knowledge_search"]["comparisons"]
    assert kg_cmp["warm_vs_no_reuse_p50_ratio"] == 0.6
    assert kg_cmp["warm_vs_default_cold_p50_delta_ms"] == -680.0

    ranking = summary["rank_by_p50_ms"]
    assert ranking[0]["tool"] == "cli.skill_run.knowledge_search.default_cold"
    assert ranking[-1]["tool"] == "cli.skill_run.default_warm"


def test_build_cli_runner_summary_handles_missing_modes() -> None:
    """When modes are missing for one profile, comparison values should be None."""
    module = _load_benchmark_script_module()
    results = [
        {
            "tool": "cli.skill_run.default_warm",
            "tool_name": "cli.skill_run",
            "p50_ms": 350.0,
            "p95_ms": 380.0,
            "avg_ms": 360.0,
            "ok": True,
        }
    ]
    summary = module._build_cli_runner_summary(results)

    assert summary["case_count"] == 1
    demo_profile = summary["profiles"]["demo_hello"]
    cmp = demo_profile["comparisons"]
    assert cmp["warm_vs_no_reuse_p50_ratio"] is None
    assert cmp["warm_vs_default_cold_p50_delta_ms"] is None


def test_build_cli_runner_summary_artifact_shape() -> None:
    """Standalone CLI summary artifact should expose schema + benchmark + summary payload."""
    module = _load_benchmark_script_module()
    summary = {
        "case_count": 2,
        "profiles": {"demo_hello": {"command": "demo.hello", "cases": {}, "comparisons": {}}},
        "rank_by_p50_ms": [],
    }
    artifact = module._build_cli_runner_summary_artifact(
        cli_runner_summary=summary,
        runs_per_tool=3,
        warm_phase=True,
        gate_scope=module.SNAPSHOT_GATE_SCOPE_DETERMINISTIC,
        checked_case_count=30,
        anomaly_count=0,
    )

    assert artifact["schema"] == module.CLI_RUNNER_SUMMARY_SCHEMA
    assert artifact["benchmark"]["runs_per_tool"] == 3
    assert artifact["benchmark"]["warm_phase"] is True
    assert artifact["snapshot"]["gate_scope"] == module.SNAPSHOT_GATE_SCOPE_DETERMINISTIC
    assert artifact["snapshot"]["checked_case_count"] == 30
    assert artifact["snapshot"]["anomaly_count"] == 0
    assert artifact["cli_runner_summary"]["case_count"] == 2


def test_evaluate_cli_runner_ordering_passes_for_expected_order() -> None:
    """Ordering gate should pass when warm < no_reuse < cold for each profile."""
    module = _load_benchmark_script_module()
    results = [
        {
            "tool": "cli.skill_run.default_cold",
            "tool_name": "cli.skill_run",
            "p50_ms": 900.0,
            "p95_ms": 980.0,
            "avg_ms": 910.0,
            "ok": True,
        },
        {
            "tool": "cli.skill_run.default_warm",
            "tool_name": "cli.skill_run",
            "p50_ms": 300.0,
            "p95_ms": 340.0,
            "avg_ms": 310.0,
            "ok": True,
        },
        {
            "tool": "cli.skill_run.no_reuse",
            "tool_name": "cli.skill_run",
            "p50_ms": 500.0,
            "p95_ms": 520.0,
            "avg_ms": 505.0,
            "ok": True,
        },
    ]
    summary = module._build_cli_runner_summary(results)
    gate = module._evaluate_cli_runner_ordering(summary)
    assert gate["violation_count"] == 0
    assert gate["checked_profile_count"] == 1
    assert gate["expected_order"] == module.CLI_RUNNER_GATE_EXPECTED_ORDER
    assert gate["tolerance_ms"] == module.CLI_RUNNER_GATE_DEFAULT_TOLERANCE_MS


def test_evaluate_cli_runner_ordering_reports_violation() -> None:
    """Ordering gate should report violation when no_reuse is slower than cold."""
    module = _load_benchmark_script_module()
    results = [
        {
            "tool": "cli.skill_run.default_cold",
            "tool_name": "cli.skill_run",
            "p50_ms": 700.0,
            "p95_ms": 710.0,
            "avg_ms": 705.0,
            "ok": True,
        },
        {
            "tool": "cli.skill_run.default_warm",
            "tool_name": "cli.skill_run",
            "p50_ms": 300.0,
            "p95_ms": 320.0,
            "avg_ms": 305.0,
            "ok": True,
        },
        {
            "tool": "cli.skill_run.no_reuse",
            "tool_name": "cli.skill_run",
            "p50_ms": 900.0,
            "p95_ms": 930.0,
            "avg_ms": 915.0,
            "ok": True,
        },
    ]
    summary = module._build_cli_runner_summary(results)
    gate = module._evaluate_cli_runner_ordering(summary)
    assert gate["violation_count"] == 1
    violation = gate["violations"][0]
    assert violation["reason"] == "ordering_violation"
    assert violation["profile"] == "demo_hello"


def test_evaluate_cli_runner_ordering_allows_small_jitter_with_tolerance() -> None:
    """Small inversion under tolerance should not trigger ordering violation."""
    module = _load_benchmark_script_module()
    results = [
        {
            "tool": "cli.skill_run.default_cold",
            "tool_name": "cli.skill_run",
            "p50_ms": 700.0,
            "p95_ms": 730.0,
            "avg_ms": 705.0,
            "ok": True,
        },
        {
            "tool": "cli.skill_run.default_warm",
            "tool_name": "cli.skill_run",
            "p50_ms": 520.0,
            "p95_ms": 550.0,
            "avg_ms": 525.0,
            "ok": True,
        },
        {
            "tool": "cli.skill_run.no_reuse",
            "tool_name": "cli.skill_run",
            "p50_ms": 500.0,
            "p95_ms": 540.0,
            "avg_ms": 510.0,
            "ok": True,
        },
    ]
    summary = module._build_cli_runner_summary(results)
    gate = module._evaluate_cli_runner_ordering(summary, tolerance_ms=50.0)
    assert gate["violation_count"] == 0


def test_parse_cli_timing_payload_extracts_prefixed_json() -> None:
    """Timing parser should read prefixed stderr line and decode JSON payload."""
    module = _load_benchmark_script_module()
    stderr_text = (
        "warn: something else\n"
        f"{module.CLI_RUNNER_TIMING_PREFIX}"
        '{"mode":"daemon","bootstrap_ms":12.3,"daemon_connect_ms":45.6,"tool_exec_ms":78.9}\n'
    )

    parsed = module._parse_cli_timing_payload(stderr_text)

    assert isinstance(parsed, dict)
    assert parsed["mode"] == "daemon"
    assert parsed["bootstrap_ms"] == 12.3
    assert parsed["daemon_connect_ms"] == 45.6
    assert parsed["tool_exec_ms"] == 78.9


def test_build_cli_runner_summary_includes_timing_breakdown() -> None:
    """Summary should carry per-case timing breakdown when benchmark provides it."""
    module = _load_benchmark_script_module()
    results = [
        {
            "tool": "cli.skill_run.default_warm",
            "tool_name": "cli.skill_run",
            "p50_ms": 300.0,
            "p95_ms": 340.0,
            "avg_ms": 310.0,
            "ok": True,
            "timing_breakdown_ms": {
                "bootstrap": {"samples": 3, "avg_ms": 12.0, "p50_ms": 11.0, "p95_ms": 14.0},
                "daemon_connect": {
                    "samples": 3,
                    "avg_ms": 60.0,
                    "p50_ms": 58.0,
                    "p95_ms": 66.0,
                },
                "tool_exec": {"samples": 3, "avg_ms": 210.0, "p50_ms": 205.0, "p95_ms": 225.0},
            },
        }
    ]

    summary = module._build_cli_runner_summary(results)
    demo_case = summary["profiles"]["demo_hello"]["cases"]["default_warm"]

    assert "timing_breakdown_ms" in demo_case
    assert demo_case["timing_breakdown_ms"]["tool_exec"]["avg_ms"] == 210.0
