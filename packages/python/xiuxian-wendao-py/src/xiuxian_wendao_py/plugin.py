"""Plugin authoring scaffold for Arrow-backed Wendao analyzers."""

from __future__ import annotations

import importlib.util
import tomllib
from dataclasses import asdict, dataclass
from enum import StrEnum
from pathlib import Path
from types import ModuleType
from typing import Any, Mapping

from .analyzer import ResultT, WendaoAnalyzer, run_analyzer
from .scaffold import (
    WendaoAnalyzerProfile,
    WendaoSampleTemplate,
    profile_defaults,
    profile_manifest_metadata,
)
from .transport import WendaoFlightRouteQuery, WendaoTransportClient


class PluginTransportKind(StrEnum):
    """Transport kinds aligned to the Rust plugin-runtime contract."""

    ARROW_FLIGHT = "arrow_flight"
    ARROW_IPC_HTTP = "arrow_ipc_http"
    LOCAL_PROCESS_ARROW_IPC = "local_process_arrow_ipc"


@dataclass(frozen=True, slots=True)
class PluginProviderSelector:
    """Python mirror of the Rust capability-to-provider selector."""

    capability_id: str
    provider: str


@dataclass(frozen=True, slots=True)
class PluginTransportEndpoint:
    """Python mirror of the Rust plugin transport endpoint record."""

    base_url: str | None = None
    route: str | None = None
    health_route: str | None = None
    timeout_secs: int | None = None


@dataclass(frozen=True, slots=True)
class PluginCapabilityBinding:
    """Python mirror of the Rust runtime binding contract."""

    selector: PluginProviderSelector
    endpoint: PluginTransportEndpoint
    transport: PluginTransportKind
    contract_version: str
    starter: Mapping[str, object] | None = None
    launch: Mapping[str, object] | None = None

    def to_dict(self) -> dict[str, object]:
        """Render a JSON-serializable contract payload."""
        payload = asdict(self)
        payload["transport"] = self.transport.value
        return payload


def build_starter_payload(
    profile: WendaoAnalyzerProfile,
    *,
    sample_template: WendaoSampleTemplate | None = None,
) -> dict[str, object]:
    """Build one runtime starter payload from a named analyzer profile."""

    defaults = profile_defaults(profile)
    template = sample_template or defaults["sample_template"]
    if not isinstance(template, WendaoSampleTemplate):
        raise TypeError("profile sample template must be a WendaoSampleTemplate")
    metadata = profile_manifest_metadata(profile)
    return {
        "profile": profile.value,
        "sample_template": template.value,
        "display_name": metadata["display_name"],
        "summary": metadata["summary"],
        "tags": metadata["tags"],
    }


def load_plugin_manifest(path: str | Path) -> dict[str, object]:
    """Load one scaffolded plugin manifest from disk."""

    manifest_path = Path(path)
    return validate_plugin_manifest(
        tomllib.loads(manifest_path.read_text(encoding="utf-8"))
    )


def validate_plugin_manifest(manifest: Mapping[str, object]) -> dict[str, object]:
    """Validate one parsed plugin manifest and return a normalized copy."""

    plugin = manifest.get("plugin")
    if not isinstance(plugin, Mapping):
        raise TypeError("plugin manifest must contain a [plugin] mapping")
    entrypoint = manifest.get("entrypoint")
    if not isinstance(entrypoint, Mapping):
        raise TypeError("plugin manifest must contain an [entrypoint] mapping")

    capability_id = plugin.get("capability_id")
    provider = plugin.get("provider")
    route = plugin.get("route")
    transport = plugin.get("transport")
    health_route = plugin.get("health_route")
    module_name = entrypoint.get("module")
    callable_name = entrypoint.get("callable")

    if not isinstance(capability_id, str):
        raise TypeError("plugin.capability_id must be a string")
    if not isinstance(provider, str):
        raise TypeError("plugin.provider must be a string")
    if not isinstance(route, str):
        raise TypeError("plugin.route must be a string")
    if not isinstance(transport, str):
        raise TypeError("plugin.transport must be a string")
    if transport not in {kind.value for kind in PluginTransportKind}:
        raise ValueError(f"unsupported plugin transport: {transport}")
    if health_route is not None and not isinstance(health_route, str):
        raise TypeError("plugin.health_route must be a string when provided")
    if not isinstance(module_name, str):
        raise TypeError("entrypoint.module must be a string")
    if not isinstance(callable_name, str):
        raise TypeError("entrypoint.callable must be a string")

    starter = manifest.get("starter")
    if starter is not None:
        if not isinstance(starter, Mapping):
            raise TypeError("plugin manifest starter section must be a mapping")
        profile = starter.get("profile")
        sample_template = starter.get("sample_template")
        display_name = starter.get("display_name")
        summary = starter.get("summary")
        tags = starter.get("tags")
        if profile is not None and not isinstance(profile, str):
            raise TypeError("starter.profile must be a string when provided")
        if sample_template is not None and not isinstance(sample_template, str):
            raise TypeError("starter.sample_template must be a string when provided")
        if display_name is not None and not isinstance(display_name, str):
            raise TypeError("starter.display_name must be a string when provided")
        if summary is not None and not isinstance(summary, str):
            raise TypeError("starter.summary must be a string when provided")
        if tags is not None:
            if not isinstance(tags, (list, tuple)) or not all(
                isinstance(tag, str) for tag in tags
            ):
                raise TypeError("starter.tags must be a list or tuple of strings")

    return dict(manifest)


def _entrypoint_section(manifest: Mapping[str, object]) -> Mapping[str, object]:
    """Return the manifest entrypoint section."""

    entrypoint = validate_plugin_manifest(manifest).get("entrypoint")
    if not isinstance(entrypoint, Mapping):
        raise TypeError("plugin manifest must contain an [entrypoint] mapping")
    return entrypoint


def _load_module_from_path(module_name: str, module_path: Path) -> ModuleType:
    """Load one Python module from an explicit filesystem path."""

    spec = importlib.util.spec_from_file_location(module_name, module_path)
    if spec is None or spec.loader is None:
        raise ImportError(f"failed to create import spec for {module_path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_manifest_entrypoint(
    manifest: Mapping[str, object] | str | Path,
    *,
    project_root: str | Path,
) -> Any:
    """Load the analyzer callable referenced by a scaffolded manifest."""

    loaded_manifest = load_plugin_manifest(manifest) if isinstance(manifest, (str, Path)) else manifest
    if not isinstance(manifest, (str, Path)):
        loaded_manifest = validate_plugin_manifest(loaded_manifest)
    entrypoint = _entrypoint_section(loaded_manifest)
    module_name = entrypoint.get("module")
    callable_name = entrypoint.get("callable")
    if not isinstance(module_name, str):
        raise TypeError("entrypoint.module must be a string")
    if not isinstance(callable_name, str):
        raise TypeError("entrypoint.callable must be a string")

    module_relative = Path(*module_name.split(".")).with_suffix(".py")
    candidates = tuple(Path(project_root).glob(f"src/*/{module_relative.as_posix()}"))
    if not candidates:
        raise FileNotFoundError(
            f"could not resolve entrypoint module {module_name!r} under {project_root}"
        )
    if len(candidates) > 1:
        raise RuntimeError(
            f"entrypoint module {module_name!r} resolved ambiguously under {project_root}"
        )
    module = _load_module_from_path(f"wendao_plugin_{module_name}", candidates[0])
    try:
        return getattr(module, callable_name)
    except AttributeError as exc:
        raise AttributeError(
            f"entrypoint callable {callable_name!r} not found in {candidates[0]}"
        ) from exc


def starter_from_manifest(manifest: Mapping[str, object]) -> dict[str, object] | None:
    """Extract starter metadata from one parsed plugin manifest."""

    validated_manifest = validate_plugin_manifest(manifest)
    starter = validated_manifest.get("starter")
    if starter is None:
        return None
    if not isinstance(starter, Mapping):
        raise TypeError("plugin manifest starter section must be a mapping")
    return dict(starter)


def build_arrow_flight_binding(
    client: WendaoTransportClient,
    *,
    capability_id: str,
    provider: str,
    query: WendaoFlightRouteQuery,
    health_route: str | None = None,
    starter: Mapping[str, object] | None = None,
    launch: Mapping[str, object] | None = None,
) -> PluginCapabilityBinding:
    """Build one Arrow Flight binding from the Python transport client."""

    return PluginCapabilityBinding(
        selector=PluginProviderSelector(
            capability_id=capability_id,
            provider=provider,
        ),
        endpoint=PluginTransportEndpoint(
            base_url=client.endpoint_url(),
            route=query.normalized_route(),
            health_route=health_route,
            timeout_secs=int(client.config.request_timeout_seconds),
        ),
        transport=PluginTransportKind.ARROW_FLIGHT,
        contract_version=client.schema_version(),
        starter=starter,
        launch=launch,
    )


@dataclass(frozen=True, slots=True)
class WendaoAnalyzerPlugin:
    """One minimal analyzer-plugin scaffold for downstream Python authors."""

    capability_id: str
    provider: str
    analyzer: WendaoAnalyzer[ResultT]
    health_route: str | None = None
    starter: Mapping[str, object] | None = None
    launch: Mapping[str, object] | None = None

    def binding_for_client(
        self,
        client: WendaoTransportClient,
        query: WendaoFlightRouteQuery,
    ) -> PluginCapabilityBinding:
        """Build the runtime binding advertised by this plugin."""

        return build_arrow_flight_binding(
            client,
            capability_id=self.capability_id,
            provider=self.provider,
            query=query,
            health_route=self.health_route,
            starter=self.starter,
            launch=self.launch,
        )

    def run(
        self,
        client: WendaoTransportClient,
        query: WendaoFlightRouteQuery,
        *,
        include_flight_info: bool = True,
        **connect_kwargs: object,
    ) -> ResultT:
        """Run the plugin analyzer against one routed Arrow query."""

        return run_analyzer(
            client,
            self.analyzer,
            query,
            include_flight_info=include_flight_info,
            **connect_kwargs,
        )


def build_profiled_analyzer_plugin(
    *,
    capability_id: str,
    provider: str,
    profile: WendaoAnalyzerProfile,
    analyzer: WendaoAnalyzer[ResultT],
    sample_template: WendaoSampleTemplate | None = None,
    health_route: str | None = None,
    launch: Mapping[str, object] | None = None,
) -> WendaoAnalyzerPlugin:
    """Build one analyzer plugin with starter payload linked from a profile."""

    return WendaoAnalyzerPlugin(
        capability_id=capability_id,
        provider=provider,
        analyzer=analyzer,
        health_route=health_route,
        starter=build_starter_payload(
            profile,
            sample_template=sample_template,
        ),
        launch=launch,
    )


def plugin_from_manifest(
    manifest: Mapping[str, object] | str | Path,
    *,
    analyzer: WendaoAnalyzer[ResultT] | None = None,
    project_root: str | Path | None = None,
    launch: Mapping[str, object] | None = None,
) -> WendaoAnalyzerPlugin:
    """Build one runtime analyzer plugin from a scaffolded manifest."""

    loaded_manifest = load_plugin_manifest(manifest) if isinstance(manifest, (str, Path)) else validate_plugin_manifest(manifest)
    plugin_section = loaded_manifest.get("plugin")
    if not isinstance(plugin_section, Mapping):
        raise TypeError("plugin manifest must contain a [plugin] mapping")
    capability_id = plugin_section.get("capability_id")
    provider = plugin_section.get("provider")
    health_route = plugin_section.get("health_route")
    if not isinstance(capability_id, str):
        raise TypeError("plugin.capability_id must be a string")
    if not isinstance(provider, str):
        raise TypeError("plugin.provider must be a string")
    if health_route is not None and not isinstance(health_route, str):
        raise TypeError("plugin.health_route must be a string when provided")
    resolved_analyzer = analyzer
    if resolved_analyzer is None:
        if project_root is None:
            raise TypeError("project_root is required when analyzer is not provided")
        resolved_analyzer = load_manifest_entrypoint(
            loaded_manifest,
            project_root=project_root,
        )

    return WendaoAnalyzerPlugin(
        capability_id=capability_id,
        provider=provider,
        analyzer=resolved_analyzer,
        health_route=health_route,
        starter=starter_from_manifest(loaded_manifest),
        launch=launch,
    )


__all__ = [
    "PluginCapabilityBinding",
    "PluginProviderSelector",
    "PluginTransportEndpoint",
    "PluginTransportKind",
    "WendaoAnalyzerPlugin",
    "build_profiled_analyzer_plugin",
    "build_arrow_flight_binding",
    "build_starter_payload",
    "load_manifest_entrypoint",
    "load_plugin_manifest",
    "plugin_from_manifest",
    "starter_from_manifest",
    "validate_plugin_manifest",
]
