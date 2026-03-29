"""Example tests demonstrating the retained Omni Test Kit features."""

from __future__ import annotations

import pytest
from xiuxian_test_kit.asserts import (
    assert_equal,
    assert_has_error,
    assert_in,
    assert_length,
    assert_response_ok,
    assert_true,
)

# Import markers and assertions from test-kit
from xiuxian_test_kit.plugin import cloud, integration, unit

from xiuxian_core.responses import ToolResponse

# =============================================================================
# Testing Layer Markers Examples
# =============================================================================


@unit
def test_with_unit_marker() -> None:
    """This is a unit test - fast, isolated."""
    assert_equal(1, 1)
    assert_true(True)


@integration
async def test_with_integration_marker() -> None:
    """This is an integration test - uses real components."""
    # Integration with real scanner would go here
    assert_true(True)


@cloud
async def test_with_cloud_marker() -> None:
    """This is a cloud test - requires external services."""
    pytest.skip("Requires external LanceDB")


# =============================================================================
# Assertion Helpers Examples
# =============================================================================


@unit
def test_assertion_helpers() -> None:
    """Demonstrate assertion helpers."""
    # Basic assertions
    assert_equal(10, 10)
    assert_in("key", {"key": "value"})
    assert_length([1, 2, 3], 3)
    assert_true(True)


@unit
def test_response_assertions() -> None:
    """Demonstrate ToolResponse assertions."""
    # Create a success response
    response = ToolResponse.success(data={"result": "ok"}, metadata={"source": "test"})
    assert_response_ok(response)

    # Create an error response
    error_response = ToolResponse.error(message="Not found", code="3001")
    assert_has_error(error_response, expected_code="3001")


# =============================================================================
# Parameterized Tests
# =============================================================================


@unit
@pytest.mark.parametrize(
    "value,expected",
    [
        (1, 1),
        (2, 2),
        (3, 3),
    ],
)
def test_parametrized(value: int, expected: int) -> None:
    """Parametrized test example."""
    assert_equal(value, expected)
