from __future__ import annotations


def test_foundation_root_no_longer_forwards_config_helpers() -> None:
    import xiuxian_foundation

    for name in (
        "PRJ_DIRS",
        "PRJ_DATA",
        "PRJ_CACHE",
        "PRJ_CONFIG",
        "PRJ_RUNTIME",
        "PRJ_PATH",
        "PRJ_CHECKPOINT",
        "get_prj_dir",
        "get_data_dir",
        "get_cache_dir",
        "get_config_dir",
        "get_runtime_dir",
        "get_setting",
        "configure_logging",
        "get_project_root",
    ):
        assert not hasattr(xiuxian_foundation, name)
