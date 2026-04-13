from __future__ import annotations

import wendao_core_lib as package


def test_root_package_exports_transport_substrate_but_not_retired_compat_symbols() -> None:
    for name in (
        "WendaoTransportClient",
        "WendaoTransportConfig",
        "WendaoTransportEndpoint",
        "WendaoTransportMode",
        "WendaoFlightRouteQuery",
        "WendaoRepoSearchRequest",
        "WendaoRerankRequestRow",
    ):
        assert hasattr(package, name)

    for name in (
        "ConfigPaths",
        "PRJ_DATA",
        "PRJ_DIRS",
        "get_config_paths",
        "get_data_dir",
        "get_setting",
        "get_skills_dir",
        "get_project_root",
    ):
        assert not hasattr(package, name)
