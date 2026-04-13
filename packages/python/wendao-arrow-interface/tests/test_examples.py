from __future__ import annotations

import subprocess
from pathlib import Path


def _package_root() -> Path:
    return Path(__file__).resolve().parents[1]


def _run_example_via_uv(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["uv", "run", "--extra", "polars", "python", *args],
        cwd=_package_root(),
        check=True,
        capture_output=True,
        text=True,
    )


def test_shipped_example_set_matches_current_freeze() -> None:
    example_names = {
        path.name for path in (_package_root() / "examples").glob("*.py") if path.is_file()
    }

    assert example_names == {"attachment_pdf_polars_workflow.py"}


def test_attachment_pdf_polars_example_runs_scripted() -> None:
    result = _run_example_via_uv("examples/attachment_pdf_polars_workflow.py")

    assert "mode= scripted" in result.stdout
    assert "query_text= architecture" in result.stdout
    assert "arrow_rows= 2" in result.stdout
    assert "polars_rows= 2" in result.stdout
    assert "top_attachment_name= design-review.pdf" in result.stdout
    assert "top_source_title= Architecture Notes" in result.stdout
    assert "top_score= 0.82" in result.stdout
    assert "recorded_calls= 1" in result.stdout
    assert "recorded_route= /search/attachments" in result.stdout


def test_attachment_pdf_polars_example_exposes_help() -> None:
    result = _run_example_via_uv("examples/attachment_pdf_polars_workflow.py", "--help")

    assert "attachment_pdf_polars_workflow.py" in result.stdout
    assert "--mode {scripted,endpoint}" in result.stdout
    assert "--ext-filter" in result.stdout
    assert "--kind-filter" in result.stdout


def test_readme_mentions_arrow_first_polars_example() -> None:
    readme = (_package_root() / "README.md").read_text(encoding="utf-8")

    assert "Arrow remains the canonical raw interchange surface" in readme
    assert "Polars is an optional example adapter" in readme
    assert "## Performance Reading" in readme
    assert "examples/attachment_pdf_polars_workflow.py" in readme
    assert "Arrow Flight for transport" in readme
    assert "uv run --extra polars python examples/attachment_pdf_polars_workflow.py" in readme
    assert "parse_dataframe" not in readme
    assert "analyze_dataframe" not in readme
