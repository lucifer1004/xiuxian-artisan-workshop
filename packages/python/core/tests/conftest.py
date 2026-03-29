"""xiuxian_core tests configuration."""

from __future__ import annotations

import asyncio

import pytest

# =============================================================================
# Test Stratification Markers
# =============================================================================
#
# Tests are categorized into three tiers:
# - unit: Pure unit tests (mock all external dependencies)
# - local: Local integration tests (real services, no network calls)
# - cloud: Cloud integration tests (requires network/CI environment)
#
# Usage:
#   pytest -m unit       # Run unit tests only
#   pytest -m local     # Run local integration tests
#   pytest -m cloud     # Run cloud integration tests
#   pytest              # Run all tests
# =============================================================================


def pytest_configure(config):
    """Configure pytest with custom markers for test stratification."""
    config.addinivalue_line(
        "markers", "unit: marks tests as pure unit tests (mock all dependencies)"
    )
    config.addinivalue_line(
        "markers", "local: marks tests as local integration tests (real services, no network)"
    )
    config.addinivalue_line(
        "markers", "cloud: marks tests as cloud integration tests (requires network/CI)"
    )
    config.addinivalue_line(
        "markers", "slow: marks tests as slow running (for performance tracking)"
    )


# Core specific fixtures are now loaded from xiuxian-test-kit-core plugin


@pytest.fixture(scope="session")
def event_loop():
    """
    Create an event loop for the test session.

    Critical for Rust singletons (lazy_static) to avoid "Event loop is closed" errors.
    """
    loop = asyncio.new_event_loop()
    yield loop
    loop.close()


from .plugins.seed_manager import pytest_configure  # noqa: F401
