"""Example tests demonstrating retained pytest marker usage."""

from __future__ import annotations

import pytest

from xiuxian_core.responses import ToolResponse

# =============================================================================
# Testing Layer Markers Examples
# =============================================================================


@pytest.mark.unit
def test_with_unit_marker() -> None:
    """This is a unit test - fast, isolated."""
    assert 1 == 1
    assert True


@pytest.mark.integration
async def test_with_integration_marker() -> None:
    """This is an integration test - uses real components."""
    # Integration with real scanner would go here
    assert True


@pytest.mark.cloud
async def test_with_cloud_marker() -> None:
    """This is a cloud test - requires external services."""
    pytest.skip("Requires external LanceDB")


# =============================================================================
# Assertion Helpers Examples
# =============================================================================


@pytest.mark.unit
def test_assertion_helpers() -> None:
    """Demonstrate plain pytest assertions."""
    assert 10 == 10
    assert "key" in {"key": "value"}
    assert len([1, 2, 3]) == 3
    assert True


@pytest.mark.unit
def test_response_assertions() -> None:
    """Demonstrate ToolResponse assertions."""
    response = ToolResponse.success(data={"result": "ok"}, metadata={"source": "test"})
    assert response.status.value == "success"
    assert response.error_message is None
    assert response.error_code is None

    error_response = ToolResponse.error(message="Not found", code="3001")
    assert error_response.error_message == "Not found"
    assert error_response.error_code == "3001"


# =============================================================================
# Parameterized Tests
# =============================================================================


@pytest.mark.unit
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
    assert value == expected
