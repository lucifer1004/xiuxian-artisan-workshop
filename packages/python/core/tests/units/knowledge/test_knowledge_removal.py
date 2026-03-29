from __future__ import annotations

import pytest


def test_librarian_is_removed() -> None:
    from xiuxian_core.knowledge.librarian import Librarian

    with pytest.raises(RuntimeError, match="Python Librarian has been removed"):
        Librarian()


def test_runtime_service_librarian_access_is_removed() -> None:
    from xiuxian_core.runtime.services import ensure_librarian, get_librarian

    with pytest.raises(RuntimeError, match="Python librarian has been removed"):
        get_librarian()

    with pytest.raises(RuntimeError, match="Python librarian has been removed"):
        ensure_librarian()
