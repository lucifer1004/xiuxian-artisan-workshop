"""Removal checks for deleted local database path config helpers."""

from __future__ import annotations

from importlib.util import find_spec

import xiuxian_foundation.config.dirs as dirs_mod


def test_database_config_package_is_removed() -> None:
    assert find_spec("xiuxian_foundation.config.database") is None


def test_database_helpers_are_not_exported() -> None:
    for name in (
        "get_checkpoint_db_path",
        "get_checkpoint_table_name",
        "get_database_path",
        "get_database_paths",
        "get_knowledge_graph_scope_key",
        "get_memory_db_path",
        "get_vector_db_path",
    ):
        assert not hasattr(dirs_mod, name)
