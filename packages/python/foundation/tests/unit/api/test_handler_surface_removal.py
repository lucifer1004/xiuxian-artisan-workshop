"""Removal guard for the deleted local handler wrapper surface."""

from __future__ import annotations

import importlib
import importlib.util


def test_handlers_module_is_removed() -> None:
    assert importlib.util.find_spec("xiuxian_foundation.api.handlers") is None


def test_decorators_module_no_longer_exports_local_handler_wrappers() -> None:
    decorators = importlib.import_module("xiuxian_foundation.api.decorators")

    assert not hasattr(decorators, "CommandHandler")
    assert not hasattr(decorators, "create_handler")
    assert not hasattr(decorators, "GraphNodeHandler")
    assert not hasattr(decorators, "graph_node")
    assert not hasattr(decorators, "ExecutionResult")
    assert not hasattr(decorators, "LoggerConfig")
    assert not hasattr(decorators, "ResultConfig")
    assert not hasattr(decorators, "ErrorStrategy")
