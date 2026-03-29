"""Unit tests for scripts/fetch_previous_skills_benchmark_artifact.py."""

from __future__ import annotations

import io
import zipfile
from functools import lru_cache
from importlib.util import module_from_spec, spec_from_file_location
from typing import TYPE_CHECKING

from xiuxian_foundation.config.prj import get_project_root

if TYPE_CHECKING:
    from types import ModuleType


@lru_cache(maxsize=1)
def _load_script_module() -> ModuleType:
    script_path = get_project_root() / "scripts" / "fetch_previous_skills_benchmark_artifact.py"
    spec = spec_from_file_location("fetch_previous_skills_benchmark_artifact_script", script_path)
    assert spec is not None
    assert spec.loader is not None
    module = module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def _zip_bytes(files: dict[str, bytes]) -> bytes:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, mode="w") as archive:
        for name, content in files.items():
            archive.writestr(name, content)
    return buffer.getvalue()


def test_select_candidate_runs_skips_current_and_applies_limit() -> None:
    module = _load_script_module()
    runs = [{"id": 300}, {"id": 200}, {"id": 100}]
    selected = module._select_candidate_runs(runs, current_run_id=200, max_candidates=1)

    assert len(selected) == 1
    assert selected[0]["id"] == 300


def test_select_artifact_by_name_ignores_expired() -> None:
    module = _load_script_module()
    artifacts = [
        {"id": 1, "name": "skills-tools-benchmark-ubuntu-latest", "expired": True},
        {"id": 2, "name": "skills-tools-benchmark-ubuntu-latest", "expired": False},
    ]
    selected = module._select_artifact_by_name(
        artifacts,
        artifact_name="skills-tools-benchmark-ubuntu-latest",
    )

    assert selected is not None
    assert selected["id"] == 2


def test_extract_member_from_zip_prefers_baseline_member() -> None:
    module = _load_script_module()
    archive = _zip_bytes(
        {
            "reports/cli_runner_summary.base.json": b'{"source":"baseline"}',
            "reports/cli_runner_summary.json": b'{"source":"summary"}',
        }
    )

    extracted = module._extract_member_from_zip(
        archive_bytes=archive,
        preferred_member="cli_runner_summary.base.json",
        fallback_member="cli_runner_summary.json",
    )

    assert extracted is not None
    member_name, content = extracted
    assert member_name.endswith("cli_runner_summary.base.json")
    assert content == b'{"source":"baseline"}'


def test_extract_member_from_zip_uses_fallback_when_preferred_missing() -> None:
    module = _load_script_module()
    archive = _zip_bytes({"reports/cli_runner_summary.json": b'{"source":"summary"}'})

    extracted = module._extract_member_from_zip(
        archive_bytes=archive,
        preferred_member="cli_runner_summary.base.json",
        fallback_member="cli_runner_summary.json",
    )

    assert extracted is not None
    member_name, content = extracted
    assert member_name.endswith("cli_runner_summary.json")
    assert content == b'{"source":"summary"}'
