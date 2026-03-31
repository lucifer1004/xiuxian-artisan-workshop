"""Scaffold helpers for downstream Arrow-backed Wendao analyzer plugins."""

from __future__ import annotations

import argparse
import json
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path


class WendaoSampleTemplate(StrEnum):
    """Named offline replay templates for scaffolded analyzers."""

    DOCS_RETRIEVAL = "docs_retrieval"
    REPO_SEARCH = "repo_search"
    CODE_SYMBOL = "code_symbol"


class WendaoAnalyzerProfile(StrEnum):
    """High-level starter profiles for downstream analyzer plugins."""

    DOCS_RETRIEVAL = "docs_retrieval"
    REPO_SEARCH = "repo_search"
    CODE_SYMBOL = "code_symbol"


@dataclass(frozen=True, slots=True)
class WendaoAnalyzerPluginManifest:
    """One minimal manifest for a Python analyzer plugin."""

    plugin_id: str
    capability_id: str
    provider: str
    route: str
    entry_module: str = "analyzer"
    entry_callable: str = "analyze"
    contract_version: str = "v1"
    transport: str = "arrow_flight"
    health_route: str | None = None
    profile: str | None = None
    sample_template: str | None = None
    display_name: str | None = None
    summary: str | None = None
    tags: tuple[str, ...] = ()

    def render_toml(self) -> str:
        """Render the plugin manifest in a stable TOML layout."""
        lines = [
            '[plugin]',
            f'plugin_id = "{self.plugin_id}"',
            f'capability_id = "{self.capability_id}"',
            f'provider = "{self.provider}"',
            f'contract_version = "{self.contract_version}"',
            f'transport = "{self.transport}"',
            f'route = "{self.route}"',
        ]
        if self.health_route is not None:
            lines.append(f'health_route = "{self.health_route}"')
        lines.extend(
            [
                "",
                "[entrypoint]",
                f'module = "{self.entry_module}"',
                f'callable = "{self.entry_callable}"',
            ]
        )
        if (
            self.profile is not None
            or self.sample_template is not None
            or self.display_name is not None
            or self.summary is not None
            or self.tags
        ):
            lines.extend(["", "[starter]"])
            if self.profile is not None:
                lines.append(f'profile = "{self.profile}"')
            if self.sample_template is not None:
                lines.append(f'sample_template = "{self.sample_template}"')
            if self.display_name is not None:
                lines.append(f'display_name = "{self.display_name}"')
            if self.summary is not None:
                lines.append(f'summary = "{self.summary}"')
            if self.tags:
                tag_list = ", ".join(f'"{tag}"' for tag in self.tags)
                lines.append(f"tags = [{tag_list}]")
        return "\n".join(lines) + "\n"


def default_sample_rows() -> list[dict[str, object]]:
    """Return the default stable offline replay payload."""

    return sample_rows_for_template(WendaoSampleTemplate.DOCS_RETRIEVAL)


def sample_rows_for_template(
    template: WendaoSampleTemplate = WendaoSampleTemplate.DOCS_RETRIEVAL,
) -> list[dict[str, object]]:
    """Return the stable offline replay payload for one named template."""

    if template is WendaoSampleTemplate.DOCS_RETRIEVAL:
        return [
            {
                "id": "doc-1",
                "path": "docs/overview.md",
                "title": "Overview",
                "score": 0.97,
                "language": "markdown",
            },
            {
                "id": "doc-2",
                "path": "docs/architecture.md",
                "title": "Architecture",
                "score": 0.89,
                "language": "markdown",
            },
        ]
    if template is WendaoSampleTemplate.REPO_SEARCH:
        return [
            {
                "id": "repo-1",
                "repo": "xiuxian-artisan-workshop",
                "path": "src/search/router.py",
                "score": 0.93,
                "language": "python",
            },
            {
                "id": "repo-2",
                "repo": "xiuxian-artisan-workshop",
                "path": "packages/rust/crates/xiuxian-wendao/src/lib.rs",
                "score": 0.87,
                "language": "rust",
            },
        ]
    return [
        {
            "id": "symbol-1",
            "symbol": "WendaoTransportClient",
            "kind": "class",
            "path": "packages/python/xiuxian-wendao-py/src/xiuxian_wendao_py/transport/client.py",
            "score": 0.95,
        },
        {
            "id": "symbol-2",
            "symbol": "run_analyzer",
            "kind": "function",
            "path": "packages/python/xiuxian-wendao-py/src/xiuxian_wendao_py/analyzer.py",
            "score": 0.9,
        },
    ]


def required_fields_for_template(template: WendaoSampleTemplate) -> tuple[str, ...]:
    """Return the stable required row fields for one sample template."""

    if template is WendaoSampleTemplate.REPO_SEARCH:
        return ("id", "repo", "path", "score", "language")
    if template is WendaoSampleTemplate.CODE_SYMBOL:
        return ("id", "symbol", "kind", "path", "score")
    return ("id", "path", "title", "score", "language")


def _normalize_sample_template(
    template: WendaoSampleTemplate | str,
) -> WendaoSampleTemplate:
    """Normalize one template input to the canonical enum form."""

    return template if isinstance(template, WendaoSampleTemplate) else WendaoSampleTemplate(template)


def validate_rows_against_template(
    rows: list[dict[str, object]],
    template: WendaoSampleTemplate | str,
) -> None:
    """Validate one replay payload against the selected sample template."""

    template = _normalize_sample_template(template)
    required_fields = required_fields_for_template(template)
    for index, row in enumerate(rows):
        missing_fields = tuple(field for field in required_fields if field not in row)
        if missing_fields:
            missing_display = ", ".join(missing_fields)
            raise ValueError(
                f"row {index} does not match {template.value}: missing {missing_display}"
            )


def analyzer_body_for_template(
    entry_callable: str,
    template: WendaoSampleTemplate,
) -> str:
    """Return one starter analyzer body aligned to the selected template."""

    if template is WendaoSampleTemplate.REPO_SEARCH:
        return "\n".join(
            [
                "from __future__ import annotations",
                "",
                "import pyarrow as pa",
                "",
                "",
                f"def {entry_callable}(table: pa.Table, context) -> dict[str, object]:",
                '    """Summarize one repo-search result table."""',
                "    rows = table.to_pylist()",
                "    languages = sorted({str(row.get('language', '')) for row in rows if row.get('language')})",
                "    repos = sorted({str(row.get('repo', '')) for row in rows if row.get('repo')})",
                "    top_paths = [str(row.get('path', '')) for row in rows[:3] if row.get('path')]",
                "    return {",
                '        "rows": table.num_rows,',
                '        "route": context.query.normalized_route(),',
                '        "repos": repos,',
                '        "languages": languages,',
                '        "top_paths": top_paths,',
                "    }",
                "",
            ]
        )
    if template is WendaoSampleTemplate.CODE_SYMBOL:
        return "\n".join(
            [
                "from __future__ import annotations",
                "",
                "import pyarrow as pa",
                "",
                "",
                f"def {entry_callable}(table: pa.Table, context) -> dict[str, object]:",
                '    """Summarize one code-symbol result table."""',
                "    rows = table.to_pylist()",
                "    symbols = [str(row.get('symbol', '')) for row in rows if row.get('symbol')]",
                "    kinds = sorted({str(row.get('kind', '')) for row in rows if row.get('kind')})",
                "    return {",
                '        "rows": table.num_rows,',
                '        "route": context.query.normalized_route(),',
                '        "symbols": symbols[:5],',
                '        "kinds": kinds,',
                "    }",
                "",
            ]
        )
    return "\n".join(
        [
            "from __future__ import annotations",
            "",
            "import pyarrow as pa",
            "",
            "",
            f"def {entry_callable}(table: pa.Table, context) -> dict[str, object]:",
            '    """Summarize one docs-retrieval result table."""',
            "    rows = table.to_pylist()",
            "    titles = [str(row.get('title', '')) for row in rows[:3] if row.get('title')]",
            "    languages = sorted({str(row.get('language', '')) for row in rows if row.get('language')})",
            "    return {",
            '        "rows": table.num_rows,',
            '        "route": context.query.normalized_route(),',
            '        "titles": titles,',
            '        "languages": languages,',
            "    }",
            "",
        ]
    )


def profile_defaults(
    profile: WendaoAnalyzerProfile,
) -> dict[str, str | WendaoSampleTemplate]:
    """Return linked defaults for one high-level analyzer profile."""

    if profile is WendaoAnalyzerProfile.REPO_SEARCH:
        return {
            "capability_id": "repo_search",
            "route": "/search/repos/main",
            "sample_template": WendaoSampleTemplate.REPO_SEARCH,
        }
    if profile is WendaoAnalyzerProfile.CODE_SYMBOL:
        return {
            "capability_id": "code_symbol",
            "route": "/search/symbols/main",
            "sample_template": WendaoSampleTemplate.CODE_SYMBOL,
        }
    return {
        "capability_id": "docs_retrieval",
        "route": "/search/docs/main",
        "sample_template": WendaoSampleTemplate.DOCS_RETRIEVAL,
    }


def profile_manifest_metadata(
    profile: WendaoAnalyzerProfile,
) -> dict[str, str | tuple[str, ...]]:
    """Return starter metadata for one high-level analyzer profile."""

    if profile is WendaoAnalyzerProfile.REPO_SEARCH:
        return {
            "display_name": "Repo Search Analyzer",
            "summary": "Analyze repository search rows from Wendao Arrow Flight queries.",
            "tags": ("repo_search", "search", "arrow_flight"),
        }
    if profile is WendaoAnalyzerProfile.CODE_SYMBOL:
        return {
            "display_name": "Code Symbol Analyzer",
            "summary": "Analyze code-symbol rows from Wendao Arrow Flight queries.",
            "tags": ("code_symbol", "symbols", "arrow_flight"),
        }
    return {
        "display_name": "Docs Retrieval Analyzer",
        "summary": "Analyze docs-retrieval rows from Wendao Arrow Flight queries.",
        "tags": ("docs_retrieval", "retrieval", "arrow_flight"),
    }


def validate_rows_against_profile(
    rows: list[dict[str, object]],
    profile: WendaoAnalyzerProfile,
) -> None:
    """Validate one replay payload against the linked profile template."""

    defaults = profile_defaults(profile)
    template = defaults["sample_template"]
    if not isinstance(template, WendaoSampleTemplate):
        raise TypeError("profile default sample template must be a WendaoSampleTemplate")
    validate_rows_against_template(rows, template)


def scaffold_analyzer_plugin(
    *,
    package_name: str,
    plugin_id: str,
    capability_id: str,
    provider: str,
    route: str,
    profile: WendaoAnalyzerProfile | None = None,
    entry_module: str = "analyzer",
    entry_callable: str = "analyze",
    health_route: str | None = None,
    sample_template: WendaoSampleTemplate = WendaoSampleTemplate.DOCS_RETRIEVAL,
) -> dict[str, str]:
    """Generate one minimal downstream analyzer-plugin project scaffold."""

    package_import = package_name.replace("-", "_")
    metadata = profile_manifest_metadata(profile) if profile is not None else {}
    manifest = WendaoAnalyzerPluginManifest(
        plugin_id=plugin_id,
        capability_id=capability_id,
        provider=provider,
        route=route,
        entry_module=entry_module,
        entry_callable=entry_callable,
        health_route=health_route,
        profile=profile.value if profile is not None else None,
        sample_template=sample_template.value,
        display_name=str(metadata["display_name"]) if "display_name" in metadata else None,
        summary=str(metadata["summary"]) if "summary" in metadata else None,
        tags=metadata["tags"] if "tags" in metadata else (),
    )
    module_path = entry_module.replace(".", "/")

    return {
        "plugin.toml": manifest.render_toml(),
        "sample_rows.json": json.dumps(
            sample_rows_for_template(sample_template),
            indent=2,
            sort_keys=True,
        )
        + "\n",
        "pyproject.toml": "\n".join(
            [
                "[project]",
                f'name = "{package_name}"',
                'version = "0.1.0"',
                'requires-python = ">=3.12"',
                'dependencies = ["xiuxian-wendao-py"]',
                "",
                "[project.scripts]",
                f'{package_import.replace("_", "-")}-run = "{package_import}.cli:main"',
                "",
                "[build-system]",
                'requires = ["hatchling"]',
                'build-backend = "hatchling.build"',
                "",
                "[tool.hatch.build.targets.wheel]",
                f'packages = ["src/{package_import}"]',
                "",
            ]
        ),
        f"src/{package_import}/__init__.py": "\n".join(
            [
                f'from .{entry_module.split(".")[0]} import {entry_callable}',
                "",
                f'__all__ = ["{entry_callable}"]',
                "",
            ]
        ),
        f"src/{package_import}/{module_path}.py": analyzer_body_for_template(
            entry_callable,
            sample_template,
        ),
        f"src/{package_import}/cli.py": "\n".join(
            [
                "from __future__ import annotations",
                "",
                "import json",
                "import os",
                "from pathlib import Path",
                "",
                "from xiuxian_wendao_py import (",
                "    WendaoFlightRouteQuery,",
                "    WendaoTransportClient,",
                "    WendaoTransportConfig,",
                "    WendaoTransportEndpoint,",
                "    load_manifest_entrypoint,",
                "    load_plugin_manifest,",
                "    plugin_from_manifest,",
                "    run_analyzer_with_mock_rows,",
                "    validate_rows_against_template,",
                ")",
                "",
                "",
                "def main() -> None:",
                '    host = os.environ.get("WENDAO_HOST", "127.0.0.1")',
                '    port = int(os.environ.get("WENDAO_PORT", "50051"))',
                "    project_root = Path(__file__).resolve().parents[2]",
                "    manifest_path = project_root / 'plugin.toml'",
                "    manifest = load_plugin_manifest(manifest_path)",
                "    plugin_config = manifest.get('plugin', {})",
                "    default_route = str(plugin_config.get('route', ''))",
                "    if not default_route:",
                "        raise ValueError('plugin.toml must define plugin.route')",
                "    route = os.environ.get('WENDAO_ROUTE', default_route)",
                "    bundled_sample = Path(__file__).resolve().parents[2] / 'sample_rows.json'",
                '    sample_json = os.environ.get("WENDAO_SAMPLE_JSON")',
                '    mock_flight = os.environ.get("WENDAO_MOCK_FLIGHT", "").lower() in {"1", "true", "yes"}',
                f'    sample_template = "{sample_template.value}"',
                "",
                "    query = WendaoFlightRouteQuery(route=route)",
                "    if sample_json or mock_flight:",
                "        sample_path = Path(sample_json) if sample_json else bundled_sample",
                "        rows = json.loads(sample_path.read_text(encoding='utf-8'))",
                "        validate_rows_against_template(rows, sample_template)",
                "        analyzer = load_manifest_entrypoint(manifest, project_root=project_root)",
                "        result = run_analyzer_with_mock_rows(analyzer, rows, query)",
                "    else:",
                "        client = WendaoTransportClient(",
                "            WendaoTransportConfig(",
                "                endpoint=WendaoTransportEndpoint(host=host, port=port),",
                "            )",
                "        )",
                "        plugin = plugin_from_manifest(manifest, project_root=project_root)",
                "        result = plugin.run(client, query)",
                "    print(json.dumps(result, indent=2, sort_keys=True))",
                "",
                "",
                'if __name__ == "__main__":',
                "    main()",
                "",
            ]
        ),
    }


def write_scaffold_project(
    target_dir: str | Path,
    *,
    package_name: str,
    plugin_id: str,
    capability_id: str,
    provider: str,
    route: str,
    profile: WendaoAnalyzerProfile | None = None,
    entry_module: str = "analyzer",
    entry_callable: str = "analyze",
    health_route: str | None = None,
    sample_template: WendaoSampleTemplate = WendaoSampleTemplate.DOCS_RETRIEVAL,
    overwrite: bool = False,
) -> list[Path]:
    """Write one scaffolded analyzer-plugin project to disk."""

    root = Path(target_dir)
    files = scaffold_analyzer_plugin(
        package_name=package_name,
        plugin_id=plugin_id,
        capability_id=capability_id,
        provider=provider,
        route=route,
        profile=profile,
        entry_module=entry_module,
        entry_callable=entry_callable,
        health_route=health_route,
        sample_template=sample_template,
    )
    written: list[Path] = []

    for relative_path, content in files.items():
        path = root / relative_path
        if path.exists() and not overwrite:
            raise FileExistsError(f"scaffold target already exists: {path}")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
        written.append(path)

    return written


def main(argv: list[str] | None = None) -> int:
    """CLI entrypoint for writing one analyzer-plugin scaffold."""

    parser = argparse.ArgumentParser(
        prog="xiuxian-wendao-scaffold",
        description="Write one Arrow-backed Wendao analyzer-plugin scaffold.",
    )
    parser.add_argument("target_dir")
    parser.add_argument("--package-name", required=True)
    parser.add_argument("--plugin-id", required=True)
    parser.add_argument("--capability-id")
    parser.add_argument("--provider", required=True)
    parser.add_argument("--route")
    parser.add_argument(
        "--profile",
        choices=[profile.value for profile in WendaoAnalyzerProfile],
    )
    parser.add_argument("--entry-module", default="analyzer")
    parser.add_argument("--entry-callable", default="analyze")
    parser.add_argument("--health-route")
    parser.add_argument(
        "--sample-template",
        choices=[template.value for template in WendaoSampleTemplate],
    )
    parser.add_argument("--overwrite", action="store_true")
    args = parser.parse_args(argv)

    defaults = (
        profile_defaults(WendaoAnalyzerProfile(args.profile))
        if args.profile
        else None
    )
    profile = WendaoAnalyzerProfile(args.profile) if args.profile else None
    capability_id = args.capability_id or (
        str(defaults["capability_id"]) if defaults else None
    )
    route = args.route or (str(defaults["route"]) if defaults else None)
    sample_template = (
        WendaoSampleTemplate(args.sample_template)
        if args.sample_template
        else (
            defaults["sample_template"]
            if defaults
            else WendaoSampleTemplate.DOCS_RETRIEVAL
        )
    )
    if capability_id is None:
        parser.error("--capability-id is required when --profile is not provided")
    if route is None:
        parser.error("--route is required when --profile is not provided")

    write_scaffold_project(
        args.target_dir,
        package_name=args.package_name,
        plugin_id=args.plugin_id,
        capability_id=capability_id,
        provider=args.provider,
        route=route,
        profile=profile,
        entry_module=args.entry_module,
        entry_callable=args.entry_callable,
        health_route=args.health_route,
        sample_template=sample_template,
        overwrite=args.overwrite,
    )
    return 0


__all__ = [
    "analyzer_body_for_template",
    "WendaoAnalyzerProfile",
    "WendaoSampleTemplate",
    "WendaoAnalyzerPluginManifest",
    "default_sample_rows",
    "main",
    "profile_defaults",
    "profile_manifest_metadata",
    "required_fields_for_template",
    "sample_rows_for_template",
    "scaffold_analyzer_plugin",
    "validate_rows_against_profile",
    "validate_rows_against_template",
    "write_scaffold_project",
]
