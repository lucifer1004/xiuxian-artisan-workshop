from __future__ import annotations


def test_config_package_does_not_re_export_helper_symbols() -> None:
    import xiuxian_foundation.config as config_pkg

    for name in (
        "PRJ_DIRS",
        "PRJ_DATA",
        "PRJ_CACHE",
        "PRJ_CONFIG",
        "PRJ_RUNTIME",
        "PRJ_PATH",
        "get_prj_dir",
        "get_data_dir",
        "get_cache_dir",
        "get_config_dir",
        "get_runtime_dir",
        "get_skills_dir",
        "get_setting",
        "get_settings",
        "Settings",
        "LinkGraphRuntimeConfig",
    ):
        assert not hasattr(config_pkg, name)
