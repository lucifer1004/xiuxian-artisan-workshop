"""Core test fixtures - paths, tracing, and utilities."""
from pathlib import Path
from unittest.mock import MagicMock

import pytest
from xiuxian_test_kit.logging import TestTracer, setup_test_logging
from xiuxian_wendao_py.compat.config import PRJ_DIRS
from xiuxian_wendao_py.compat.runtime import get_project_root


@pytest.fixture(scope="session", autouse=True)
def _test_logging():
    setup_test_logging()


@pytest.fixture
def test_tracer(request):
    """Fixture to provide a TestTracer instance."""
    return TestTracer(request.node.name)


@pytest.fixture
def project_root() -> Path:
    """Get the project root directory using git toplevel."""
    root = get_project_root()
    assert root.exists(), f"Project root not found: {root}"
    return root


@pytest.fixture
def config_dir() -> Path:
    """Get the config directory (PRJ_CONFIG_HOME)."""
    return PRJ_DIRS.config_home


@pytest.fixture
def cache_dir() -> Path:
    """Get the cache directory (PRJ_CACHE_HOME)."""
    return PRJ_DIRS.cache_home


@pytest.fixture
def clean_settings():
    """
    Fixture that resets Settings singleton before and after test.
    Returns a fresh Settings instance.
    """
    from xiuxian_foundation.config.settings import Settings

    # Save original state
    original_instance = Settings._instance
    original_loaded = Settings._loaded

    # Reset
    Settings._instance = None
    Settings._loaded = False

    yield Settings()

    # Restore
    Settings._instance = original_instance
    Settings._loaded = original_loaded


@pytest.fixture
def mock_agent_context():
    ctx = MagicMock()
    ctx.memory = MagicMock()
    ctx.logger = MagicMock()
    return ctx
