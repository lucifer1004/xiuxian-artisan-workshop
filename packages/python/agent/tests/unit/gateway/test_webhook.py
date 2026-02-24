"""Tests for decommissioned Python gateway webhook entrypoint."""

from __future__ import annotations

import pytest

from omni.agent.gateway import create_webhook_app


def test_create_webhook_app_is_decommissioned() -> None:
    with pytest.raises(RuntimeError, match="decommissioned"):
        create_webhook_app(kernel=object(), enable_cors=False)
