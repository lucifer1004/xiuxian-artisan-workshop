from __future__ import annotations

from xiuxian_wendao_py import (
    PRJ_DATA,
    PRJ_DIRS,
    get_config_paths,
    get_data_dir,
    get_setting,
)


def test_compat_config_exports_are_available() -> None:
    paths = get_config_paths()

    assert callable(PRJ_DATA)
    assert hasattr(PRJ_DIRS, "data_home")
    assert get_data_dir().name == ".data"
    assert paths.project_root.name == "xiuxian-artisan-workshop"
    assert get_setting("link_graph.backend", "wendao") is not None
