from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

from xiuxian_wendao_py.scaffold import (
    analyzer_body_for_template,
    profile_defaults,
    profile_manifest_metadata,
    required_fields_for_template,
    validate_rows_against_profile,
    validate_rows_against_template,
    WendaoAnalyzerProfile,
    WendaoSampleTemplate,
    WendaoAnalyzerPluginManifest,
    default_sample_rows,
    main,
    sample_rows_for_template,
    scaffold_analyzer_plugin,
    write_scaffold_project,
)


def test_manifest_renders_stable_toml() -> None:
    manifest = WendaoAnalyzerPluginManifest(
        plugin_id="acme.repo_search",
        capability_id="repo_search",
        provider="acme.python.analyzer",
        route="/search/repos/main",
        health_route="/healthz",
        profile="repo_search",
        sample_template="repo_search",
        display_name="Repo Search Analyzer",
        summary="Analyze repository search rows from Wendao Arrow Flight queries.",
        tags=("repo_search", "search", "arrow_flight"),
    )

    rendered = manifest.render_toml()

    assert 'plugin_id = "acme.repo_search"' in rendered
    assert 'capability_id = "repo_search"' in rendered
    assert 'provider = "acme.python.analyzer"' in rendered
    assert 'transport = "arrow_flight"' in rendered
    assert 'route = "/search/repos/main"' in rendered
    assert 'health_route = "/healthz"' in rendered
    assert "[entrypoint]" in rendered
    assert 'module = "analyzer"' in rendered
    assert 'callable = "analyze"' in rendered
    assert "[starter]" in rendered
    assert 'profile = "repo_search"' in rendered
    assert 'sample_template = "repo_search"' in rendered
    assert 'display_name = "Repo Search Analyzer"' in rendered
    assert (
        'summary = "Analyze repository search rows from Wendao Arrow Flight queries."' in rendered
    )
    assert 'tags = ["repo_search", "search", "arrow_flight"]' in rendered


def test_scaffold_analyzer_plugin_generates_minimal_project_files() -> None:
    files = scaffold_analyzer_plugin(
        package_name="acme-repo-search",
        plugin_id="acme.repo_search",
        capability_id="repo_search",
        provider="acme.python.analyzer",
        route="/search/repos/main",
    )

    assert sorted(files) == [
        "plugin.toml",
        "pyproject.toml",
        "sample_rows.json",
        "src/acme_repo_search/__init__.py",
        "src/acme_repo_search/analyzer.py",
        "src/acme_repo_search/cli.py",
    ]
    assert 'name = "acme-repo-search"' in files["pyproject.toml"]
    assert 'dependencies = ["xiuxian-wendao-py"]' in files["pyproject.toml"]
    assert 'acme-repo-search-run = "acme_repo_search.cli:main"' in files["pyproject.toml"]
    assert 'plugin_id = "acme.repo_search"' in files["plugin.toml"]
    assert 'route = "/search/repos/main"' in files["plugin.toml"]
    assert "def analyze(table: pa.Table, context)" in files["src/acme_repo_search/analyzer.py"]
    assert "manifest_path = project_root / 'plugin.toml'" in files["src/acme_repo_search/cli.py"]
    assert "manifest = load_plugin_manifest(manifest_path)" in files["src/acme_repo_search/cli.py"]
    assert (
        "default_route = str(plugin_config.get('route', ''))"
        in files["src/acme_repo_search/cli.py"]
    )
    assert (
        "route = os.environ.get('WENDAO_ROUTE', default_route)"
        in files["src/acme_repo_search/cli.py"]
    )
    assert (
        "bundled_sample = Path(__file__).resolve().parents[2] / 'sample_rows.json'"
        in files["src/acme_repo_search/cli.py"]
    )
    assert (
        'sample_json = os.environ.get("WENDAO_SAMPLE_JSON")' in files["src/acme_repo_search/cli.py"]
    )
    assert (
        'mock_flight = os.environ.get("WENDAO_MOCK_FLIGHT", "").lower() in {"1", "true", "yes"}'
        in files["src/acme_repo_search/cli.py"]
    )
    assert 'sample_template = "docs_retrieval"' in files["src/acme_repo_search/cli.py"]
    assert (
        "sample_path = Path(sample_json) if sample_json else bundled_sample"
        in files["src/acme_repo_search/cli.py"]
    )
    assert (
        "validate_rows_against_template(rows, sample_template)"
        in files["src/acme_repo_search/cli.py"]
    )
    assert (
        "rows = json.loads(sample_path.read_text(encoding='utf-8'))"
        in files["src/acme_repo_search/cli.py"]
    )
    assert (
        "analyzer = load_manifest_entrypoint(manifest, project_root=project_root)"
        in files["src/acme_repo_search/cli.py"]
    )
    assert (
        "result = run_analyzer_with_mock_rows(analyzer, rows, query)"
        in files["src/acme_repo_search/cli.py"]
    )
    assert (
        "plugin = plugin_from_manifest(manifest, project_root=project_root)"
        in files["src/acme_repo_search/cli.py"]
    )
    assert "result = plugin.run(client, query)" in files["src/acme_repo_search/cli.py"]
    assert '"id": "doc-1"' in files["sample_rows.json"]
    assert '"language": "markdown"' in files["sample_rows.json"]
    assert '"titles": titles' in files["src/acme_repo_search/analyzer.py"]
    assert '"languages": languages' in files["src/acme_repo_search/analyzer.py"]


def test_default_sample_rows_has_stable_contract() -> None:
    rows = default_sample_rows()

    assert rows == [
        {
            "id": "doc-1",
            "language": "markdown",
            "path": "docs/overview.md",
            "score": 0.97,
            "title": "Overview",
        },
        {
            "id": "doc-2",
            "language": "markdown",
            "path": "docs/architecture.md",
            "score": 0.89,
            "title": "Architecture",
        },
    ]


def test_sample_rows_for_repo_search_template_has_stable_contract() -> None:
    rows = sample_rows_for_template(WendaoSampleTemplate.REPO_SEARCH)

    assert rows == [
        {
            "id": "repo-1",
            "language": "python",
            "path": "src/search/router.py",
            "repo": "xiuxian-artisan-workshop",
            "score": 0.93,
        },
        {
            "id": "repo-2",
            "language": "rust",
            "path": "packages/rust/crates/xiuxian-wendao/src/lib.rs",
            "repo": "xiuxian-artisan-workshop",
            "score": 0.87,
        },
    ]


def test_sample_rows_for_code_symbol_template_has_stable_contract() -> None:
    rows = sample_rows_for_template(WendaoSampleTemplate.CODE_SYMBOL)

    assert rows == [
        {
            "id": "symbol-1",
            "kind": "class",
            "path": "packages/python/xiuxian-wendao-py/src/xiuxian_wendao_py/transport/client.py",
            "score": 0.95,
            "symbol": "WendaoTransportClient",
        },
        {
            "id": "symbol-2",
            "kind": "function",
            "path": "packages/python/xiuxian-wendao-py/src/xiuxian_wendao_py/analyzer.py",
            "score": 0.9,
            "symbol": "run_analyzer",
        },
    ]


def test_scaffold_analyzer_plugin_uses_selected_sample_template() -> None:
    files = scaffold_analyzer_plugin(
        package_name="acme-repo-search",
        plugin_id="acme.repo_search",
        capability_id="repo_search",
        provider="acme.python.analyzer",
        route="/search/repos/main",
        sample_template=WendaoSampleTemplate.REPO_SEARCH,
    )

    assert '"repo": "xiuxian-artisan-workshop"' in files["sample_rows.json"]
    assert '"language": "rust"' in files["sample_rows.json"]
    assert 'sample_template = "repo_search"' in files["src/acme_repo_search/cli.py"]
    assert '"repos": repos' in files["src/acme_repo_search/analyzer.py"]
    assert '"top_paths": top_paths' in files["src/acme_repo_search/analyzer.py"]


def test_analyzer_body_for_code_symbol_template_has_profile_specific_logic() -> None:
    rendered = analyzer_body_for_template("analyze", WendaoSampleTemplate.CODE_SYMBOL)

    assert '"symbols": symbols[:5]' in rendered
    assert '"kinds": kinds' in rendered
    assert "row.get('symbol'" in rendered


def test_profile_defaults_for_repo_search_are_linked() -> None:
    defaults = profile_defaults(WendaoAnalyzerProfile.REPO_SEARCH)

    assert defaults == {
        "capability_id": "repo_search",
        "route": "/search/repos/main",
        "sample_template": WendaoSampleTemplate.REPO_SEARCH,
    }


def test_profile_defaults_for_code_symbol_are_linked() -> None:
    defaults = profile_defaults(WendaoAnalyzerProfile.CODE_SYMBOL)

    assert defaults == {
        "capability_id": "code_symbol",
        "route": "/search/symbols/main",
        "sample_template": WendaoSampleTemplate.CODE_SYMBOL,
    }


def test_profile_manifest_metadata_for_repo_search_is_stable() -> None:
    metadata = profile_manifest_metadata(WendaoAnalyzerProfile.REPO_SEARCH)

    assert metadata == {
        "display_name": "Repo Search Analyzer",
        "summary": "Analyze repository search rows from Wendao Arrow Flight queries.",
        "tags": ("repo_search", "search", "arrow_flight"),
    }


def test_required_fields_for_repo_search_template_are_stable() -> None:
    assert required_fields_for_template(WendaoSampleTemplate.REPO_SEARCH) == (
        "id",
        "repo",
        "path",
        "score",
        "language",
    )


def test_validate_rows_against_template_accepts_valid_repo_rows() -> None:
    validate_rows_against_template(
        sample_rows_for_template(WendaoSampleTemplate.REPO_SEARCH),
        WendaoSampleTemplate.REPO_SEARCH,
    )


def test_validate_rows_against_profile_rejects_missing_fields() -> None:
    try:
        validate_rows_against_profile(
            [{"id": "repo-1", "path": "src/search/router.py"}],
            WendaoAnalyzerProfile.REPO_SEARCH,
        )
    except ValueError as exc:
        assert "missing repo, score, language" in str(exc)
    else:
        raise AssertionError("expected ValueError")


def test_write_scaffold_project_writes_files_to_disk(tmp_path: Path) -> None:
    written = write_scaffold_project(
        tmp_path / "acme-repo-search",
        package_name="acme-repo-search",
        plugin_id="acme.repo_search",
        capability_id="repo_search",
        provider="acme.python.analyzer",
        route="/search/repos/main",
    )

    written_set = {path.relative_to(tmp_path / "acme-repo-search").as_posix() for path in written}
    assert written_set == {
        "plugin.toml",
        "pyproject.toml",
        "sample_rows.json",
        "src/acme_repo_search/__init__.py",
        "src/acme_repo_search/analyzer.py",
        "src/acme_repo_search/cli.py",
    }
    assert (
        (tmp_path / "acme-repo-search" / "plugin.toml")
        .read_text(encoding="utf-8")
        .startswith("[plugin]\n")
    )
    assert (
        (tmp_path / "acme-repo-search" / "sample_rows.json")
        .read_text(encoding="utf-8")
        .startswith("[\n")
    )


def test_write_scaffold_project_rejects_existing_files_without_overwrite(tmp_path: Path) -> None:
    target = tmp_path / "acme-repo-search"
    write_scaffold_project(
        target,
        package_name="acme-repo-search",
        plugin_id="acme.repo_search",
        capability_id="repo_search",
        provider="acme.python.analyzer",
        route="/search/repos/main",
    )

    try:
        write_scaffold_project(
            target,
            package_name="acme-repo-search",
            plugin_id="acme.repo_search",
            capability_id="repo_search",
            provider="acme.python.analyzer",
            route="/search/repos/main",
        )
    except FileExistsError as exc:
        assert "scaffold target already exists" in str(exc)
    else:
        raise AssertionError("expected FileExistsError")


def test_scaffold_main_writes_target_directory(tmp_path: Path) -> None:
    exit_code = main(
        [
            str(tmp_path / "acme-repo-search"),
            "--package-name",
            "acme-repo-search",
            "--plugin-id",
            "acme.repo_search",
            "--capability-id",
            "repo_search",
            "--provider",
            "acme.python.analyzer",
            "--route",
            "/search/repos/main",
            "--sample-template",
            "repo_search",
        ]
    )

    assert exit_code == 0
    assert (tmp_path / "acme-repo-search" / "src" / "acme_repo_search" / "cli.py").exists()
    assert '"repo": "xiuxian-artisan-workshop"' in (
        tmp_path / "acme-repo-search" / "sample_rows.json"
    ).read_text(encoding="utf-8")


def test_scaffold_main_uses_profile_defaults_without_capability_or_route(tmp_path: Path) -> None:
    exit_code = main(
        [
            str(tmp_path / "acme-repo-search"),
            "--package-name",
            "acme-repo-search",
            "--plugin-id",
            "acme.repo_search",
            "--provider",
            "acme.python.analyzer",
            "--profile",
            "repo_search",
        ]
    )

    assert exit_code == 0
    plugin_toml = (tmp_path / "acme-repo-search" / "plugin.toml").read_text(encoding="utf-8")
    sample_rows = (tmp_path / "acme-repo-search" / "sample_rows.json").read_text(encoding="utf-8")
    analyzer_code = (
        tmp_path / "acme-repo-search" / "src" / "acme_repo_search" / "analyzer.py"
    ).read_text(encoding="utf-8")

    assert 'capability_id = "repo_search"' in plugin_toml
    assert 'route = "/search/repos/main"' in plugin_toml
    assert "[starter]" in plugin_toml
    assert 'profile = "repo_search"' in plugin_toml
    assert 'sample_template = "repo_search"' in plugin_toml
    assert 'display_name = "Repo Search Analyzer"' in plugin_toml
    assert '"repo": "xiuxian-artisan-workshop"' in sample_rows
    assert '"repos": repos' in analyzer_code


def test_scaffolded_project_cli_runs_in_mock_flight_mode(tmp_path: Path) -> None:
    project_root = tmp_path / "acme-repo-search"
    write_scaffold_project(
        project_root,
        package_name="acme-repo-search",
        plugin_id="acme.repo_search",
        capability_id="repo_search",
        provider="acme.python.analyzer",
        route="/search/repos/main",
        profile=WendaoAnalyzerProfile.REPO_SEARCH,
        sample_template=WendaoSampleTemplate.REPO_SEARCH,
    )

    package_src = project_root / "src"
    sdk_src = Path(
        "/Users/guangtao/projects/xiuxian-artisan-workshop/packages/python/xiuxian-wendao-py/src"
    )
    existing_pythonpath = os.environ.get("PYTHONPATH", "")
    pythonpath_parts = [str(package_src), str(sdk_src)]
    if existing_pythonpath:
        pythonpath_parts.append(existing_pythonpath)

    env = os.environ.copy()
    env["PYTHONPATH"] = os.pathsep.join(pythonpath_parts)
    env["WENDAO_MOCK_FLIGHT"] = "1"

    completed = subprocess.run(
        [sys.executable, "-m", "acme_repo_search.cli"],
        cwd=project_root,
        env=env,
        capture_output=True,
        text=True,
        check=True,
    )
    payload = json.loads(completed.stdout)

    assert payload == {
        "languages": ["python", "rust"],
        "repos": ["xiuxian-artisan-workshop"],
        "route": "/search/repos/main",
        "rows": 2,
        "top_paths": [
            "src/search/router.py",
            "packages/rust/crates/xiuxian-wendao/src/lib.rs",
        ],
    }


def test_scaffolded_project_cli_uses_manifest_route_as_single_source_of_truth(
    tmp_path: Path,
) -> None:
    project_root = tmp_path / "acme-repo-search"
    write_scaffold_project(
        project_root,
        package_name="acme-repo-search",
        plugin_id="acme.repo_search",
        capability_id="repo_search",
        provider="acme.python.analyzer",
        route="/search/repos/main",
        profile=WendaoAnalyzerProfile.REPO_SEARCH,
        sample_template=WendaoSampleTemplate.REPO_SEARCH,
    )
    manifest_path = project_root / "plugin.toml"
    manifest_path.write_text(
        manifest_path.read_text(encoding="utf-8").replace(
            "/search/repos/main", "/search/repos/manifest-driven"
        ),
        encoding="utf-8",
    )

    package_src = project_root / "src"
    sdk_src = Path(
        "/Users/guangtao/projects/xiuxian-artisan-workshop/packages/python/xiuxian-wendao-py/src"
    )
    env = os.environ.copy()
    env["PYTHONPATH"] = os.pathsep.join([str(package_src), str(sdk_src)])
    env["WENDAO_MOCK_FLIGHT"] = "1"

    completed = subprocess.run(
        [sys.executable, "-m", "acme_repo_search.cli"],
        cwd=project_root,
        env=env,
        capture_output=True,
        text=True,
        check=True,
    )
    payload = json.loads(completed.stdout)

    assert payload["route"] == "/search/repos/manifest-driven"
