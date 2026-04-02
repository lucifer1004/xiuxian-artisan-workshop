from __future__ import annotations

from pathlib import Path


def _package_root() -> Path:
    return Path(__file__).resolve().parents[1]


def test_first_analyzer_author_tutorial_mentions_supported_workflows() -> None:
    tutorial = (_package_root() / "docs" / "first_analyzer_author_tutorial.md").read_text(
        encoding="utf-8"
    )

    assert "Workflow 1: Local Rerank Experiment" in tutorial
    assert "Workflow 2: Host-Backed Repo Search With Built-In Ranking" in tutorial
    assert "Workflow 3: Host-Backed Repo Search With A Custom Python Analyzer" in tutorial
    assert "examples/local_rerank_workflow.py" in tutorial
    assert "examples/repo_search_workflow.py" in tutorial
    assert "examples/custom_repo_analyzer_workflow.py" in tutorial
    assert "run_repo_analysis(...)" in tutorial
    assert "run_rerank_analysis(...)" in tutorial


def test_custom_analyzer_tutorial_mentions_contract_and_example() -> None:
    tutorial = (_package_root() / "docs" / "write_your_first_custom_analyzer.md").read_text(
        encoding="utf-8"
    )

    assert "The Smallest Honest Contract" in tutorial
    assert "def analyze_rows(self, rows: list[dict[str, object]])" in tutorial
    assert "include a stable `rank` field" in tutorial
    assert "custom_repo_analyzer_workflow.py" in tutorial
    assert "run_repo_analysis(...)" in tutorial
    assert "summarize_repo_analysis(...)" in tutorial


def test_release_policy_mentions_beta_contract_and_workflow_stability() -> None:
    policy = (_package_root() / "docs" / "release_and_compatibility_policy.md").read_text(
        encoding="utf-8"
    )

    assert "This package is currently in beta." in policy
    assert "Compatibility Rule For This Beta" in policy
    assert "The current lockable beta baseline is `0.1.1`." in policy
    assert "run_repo_analysis(...)" in policy
    assert "run_rerank_exchange_analysis(...)" in policy
    assert "run_rerank_analysis(...)" in policy
    assert "a permanent guarantee for every helper-shaped convenience symbol" in policy
    assert "Current Beta Exit Reading" in policy
    assert "Current Beta Freeze Reading" in policy
    assert "workflow-frozen, not helper-frozen" in policy
    assert "live-host custom Python analyzer rerank workflow above the current" in policy
    assert "substrate-level live `/rerank/flight` exchange through `xiuxian-wendao-py`" in policy


def test_external_consumer_checklist_mentions_environment_and_skip_paths() -> None:
    checklist = (_package_root() / "docs" / "external_consumer_checklist.md").read_text(
        encoding="utf-8"
    )

    assert "Python `>=3.12`" in checklist
    assert "pyarrow>=14.0.0" in checklist
    assert "uv run python examples/local_rerank_workflow.py" in checklist
    assert "plain `python examples/...`" in checklist
    assert "wendao_search_flight_server" in checklist
    assert (
        "cargo build -p xiuxian-wendao --features julia --bin wendao_search_flight_server --bin wendao_search_seed_sample"
        in checklist
    )
    assert 'tmp_root="$(mktemp -d)"' in checklist
    assert 'wendao_search_seed_sample alpha/repo "$tmp_root"' in checklist
    assert "examples/repo_search_workflow.py --host 127.0.0.1 --port 8815" in checklist
    assert "examples/host_backed_repo_search_beta_smoke.py --port 0" in checklist
    assert "examples/host_backed_repo_search_beta_smoke.py --mode custom --port 0" in checklist
    assert "examples/host_backed_repo_search_beta_smoke.py --port 0 --keep-workspace" in checklist
    assert "Use `--keep-workspace`" in checklist
    assert "host-backed rerank exchange through a live runtime Flight host" in checklist
    assert "run_rerank_exchange_analysis(...)" in checklist
    assert "rerank_exchange_workflow.py" in checklist
    assert "host_backed_rerank_beta_smoke.py --port 0" in checklist
    assert "live-host custom Python analyzer rerank workflow" in checklist
    assert "run one shipped example unchanged" in checklist


def test_readme_freezes_v1_documentation_set() -> None:
    readme = (_package_root() / "README.md").read_text(encoding="utf-8")

    assert "## Documentation Set" in readme
    assert "docs/first_analyzer_author_tutorial.md" in readme
    assert "docs/write_your_first_custom_analyzer.md" in readme
    assert "docs/release_and_compatibility_policy.md" in readme
    assert "docs/external_consumer_checklist.md" in readme
    assert "intended v1 public" in readme
    assert "docs surface" in readme


def test_readme_marks_package_as_beta_not_planned() -> None:
    readme = (_package_root() / "README.md").read_text(encoding="utf-8")

    assert "is a beta Python analyzer-layer package" in readme
    assert "Current scope for this beta slice" in readme
    assert "The current package boundary is now intentionally lockable as `0.1.1`." in readme


def test_readme_records_beta_readiness_and_known_gaps() -> None:
    readme = (_package_root() / "README.md").read_text(encoding="utf-8")

    assert "## Beta Readiness" in readme
    assert "ready now:" in readme
    assert "known gaps before broader adoption:" in readme
    assert "uv run python ...` from the package directory" in readme
    assert "host-backed rerank exchange through live `/rerank/flight`" in readme
    assert "no live-host custom Python analyzer rerank workflow" in readme
    assert "no GA-level release promise yet" in readme


def test_readme_records_beta_exit_audit() -> None:
    readme = (_package_root() / "README.md").read_text(encoding="utf-8")

    assert "## Beta Exit Audit" in readme
    assert "exit-ready now:" in readme
    assert "not exit-ready yet:" in readme
    assert "one-shot host-backed beta smoke" in readme
    assert "the substrate already proves a live `/rerank/flight` transport route" in readme
    assert "run_rerank_exchange_analysis(...)" in readme
    assert "runnable examples for all four workflow paths" in readme
    assert "live-host custom Python analyzer rerank workflow" in readme
    assert "current beta-exit gate:" in readme


def test_readme_records_beta_freeze_audit() -> None:
    readme = (_package_root() / "README.md").read_text(encoding="utf-8")

    assert "## Beta Freeze Audit" in readme
    assert "frozen for this beta trial:" in readme
    assert "examples/local_rerank_workflow.py" in readme
    assert "examples/host_backed_rerank_beta_smoke.py" in readme
    assert "not frozen for this beta trial:" in readme
    assert "current freeze rule:" in readme


def test_readme_mentions_host_backed_beta_smoke_example() -> None:
    readme = (_package_root() / "README.md").read_text(encoding="utf-8")

    assert "examples/host_backed_repo_search_beta_smoke.py" in readme
    assert "examples/rerank_exchange_workflow.py" in readme
    assert "examples/host_backed_rerank_beta_smoke.py" in readme
    assert "one-shot beta smoke for the full host-backed path" in readme
    assert "--mode custom --port 0" in readme
    assert "--keep-workspace" in readme
