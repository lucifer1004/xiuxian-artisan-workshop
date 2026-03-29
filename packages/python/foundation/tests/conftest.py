"""
Foundation Test Configuration

Shared fixtures for xiuxian_foundation tests.
"""

import sys
from pathlib import Path

import pytest

# Ensure xiuxian_foundation is importable if running independently
_foundation_path = Path(__file__).parent.parent.parent
if str(_foundation_path) not in sys.path:
    sys.path.insert(0, str(_foundation_path))


@pytest.fixture(scope="session")
def project_root():
    """Get project root directory."""
    from xiuxian_foundation.runtime.gitops import get_project_root

    return get_project_root()


@pytest.fixture(scope="session")
def skills_dir(project_root):
    """Get skills directory."""
    from xiuxian_foundation.config.dirs import get_skills_dir

    return get_skills_dir()


@pytest.fixture
def temp_skills_dir(tmp_path):
    """Create a temporary skills directory structure."""
    skills_dir = tmp_path / "skills"
    skills_dir.mkdir()
    (skills_dir / "git").mkdir()
    (skills_dir / "python").mkdir()
    return skills_dir


@pytest.fixture
def temp_config_dir(tmp_path):
    """Create a temporary config directory."""
    config_dir = tmp_path / ".omni"
    config_dir.mkdir()
    return config_dir


@pytest.fixture
def mock_embedding_for_search():
    """Mock embedding so search tests don't need a real embedding server.

    Patches the embedding HTTP/client helpers used by retained search helpers.
    """
    from unittest.mock import AsyncMock, MagicMock, patch

    async def _mock_embed_batch(texts, timeout_seconds=None):
        dim = 1024
        return [[0.1] * dim for _ in texts]

    mock_client = MagicMock()
    mock_client.embed_batch = AsyncMock(side_effect=_mock_embed_batch)

    mock_embed_svc = MagicMock()
    mock_embed_svc.embed.return_value = [[0.1] * 1024]

    with (
        patch(
            "xiuxian_foundation.embedding_client.get_embedding_client",
            return_value=mock_client,
        ),
        patch(
            "xiuxian_foundation.services.embedding.get_embedding_service",
            return_value=mock_embed_svc,
        ),
    ):
        yield
