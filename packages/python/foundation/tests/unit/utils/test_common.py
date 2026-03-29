from __future__ import annotations

from xiuxian_foundation.utils.common import project_root


def test_common_exports_project_root_only() -> None:
    assert callable(project_root)


def test_common_no_longer_exports_agent_path_helpers() -> None:
    import xiuxian_foundation.utils.common as common

    assert not hasattr(common, "agent_src")
    assert not hasattr(common, "setup_import_paths")
