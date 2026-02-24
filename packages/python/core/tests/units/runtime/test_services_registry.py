"""Tests for runtime service registry behavior."""

from __future__ import annotations

from unittest.mock import patch

import pytest

from omni.core.runtime.services import ServiceRegistry


@pytest.fixture(autouse=True)
def _reset_service_registry() -> None:
    """Ensure singleton registry state does not leak across tests."""
    ServiceRegistry.clear()
    yield
    ServiceRegistry.clear()


def test_missing_service_log_is_emitted_once_per_name() -> None:
    """Repeated lookups of same missing service should not spam debug logs."""
    with patch("omni.core.runtime.services.logger.debug") as debug:
        assert ServiceRegistry.get("librarian") is None
        assert ServiceRegistry.get("librarian") is None
        assert ServiceRegistry.get("librarian") is None

    debug.assert_called_once_with("Service '%s' requested but not found in registry.", "librarian")


def test_missing_log_cache_resets_after_unregister() -> None:
    """A service that was registered/unregistered should log once again when missing."""
    ServiceRegistry.register("librarian", object())
    ServiceRegistry.unregister("librarian")

    with patch("omni.core.runtime.services.logger.debug") as debug:
        assert ServiceRegistry.get("librarian") is None
        assert ServiceRegistry.get("librarian") is None

    debug.assert_called_once_with("Service '%s' requested but not found in registry.", "librarian")


def test_clear_resets_missing_log_cache() -> None:
    """Clearing registry should clear missing-service log suppression state."""
    with patch("omni.core.runtime.services.logger.debug") as debug:
        assert ServiceRegistry.get("embedding") is None
        ServiceRegistry.clear()
        assert ServiceRegistry.get("embedding") is None

    missing_calls = [
        call
        for call in debug.call_args_list
        if call.args == ("Service '%s' requested but not found in registry.", "embedding")
    ]
    assert len(missing_calls) == 2
