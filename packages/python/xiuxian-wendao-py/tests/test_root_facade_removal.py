from __future__ import annotations

import xiuxian_wendao_py as package


def test_root_package_no_longer_reexports_transport_or_compat_symbols() -> None:
    for name in (
        "ConfigPaths",
        "PRJ_DATA",
        "PRJ_DIRS",
        "WendaoRuntimeConfig",
        "WendaoTransportClient",
        "WendaoTransportConfig",
        "WendaoTransportEndpoint",
        "WendaoTransportMode",
        "get_config_paths",
        "get_data_dir",
        "get_setting",
        "get_skills_dir",
        "get_project_root",
    ):
        assert not hasattr(package, name)
