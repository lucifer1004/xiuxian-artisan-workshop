from __future__ import annotations

import importlib
from pathlib import Path


def _skill_root() -> Path:
    return Path(__file__).resolve().parents[1]


def test_neural_matrix_construct_does_not_initialize_store(monkeypatch, tmp_path) -> None:
    """Construction should remain cheap; Rust store is initialized lazily."""
    monkeypatch.syspath_prepend(str(_skill_root()))
    matrix_module = importlib.import_module("extensions.rust_bridge.matrix")

    init_calls: list[tuple[str, int, bool]] = []

    class FakeStore:
        def __init__(self, db_path: str, dimension: int, *, enable_keyword_index: bool):
            init_calls.append((db_path, dimension, enable_keyword_index))

    monkeypatch.setattr(
        matrix_module.RustBindings,
        "get_store_class",
        classmethod(lambda cls: FakeStore),
    )
    monkeypatch.setattr(
        "omni.foundation.config.database.get_memory_db_path",
        lambda: tmp_path / "memory.hippocampus.lance",
    )

    matrix = matrix_module.NeuralMatrix()
    assert matrix.backend == "omni-vector (Rust/LanceDB)"
    assert init_calls == []


def test_neural_matrix_lazily_initializes_once(monkeypatch, tmp_path) -> None:
    """First active operation initializes once and subsequent calls reuse the store."""
    monkeypatch.syspath_prepend(str(_skill_root()))
    matrix_module = importlib.import_module("extensions.rust_bridge.matrix")

    init_calls: list[tuple[str, int, bool]] = []

    class FakeStore:
        def __init__(self, db_path: str, dimension: int, *, enable_keyword_index: bool):
            init_calls.append((db_path, dimension, enable_keyword_index))

        async def health_check(self) -> bool:
            return True

    monkeypatch.setattr(
        matrix_module.RustBindings,
        "get_store_class",
        classmethod(lambda cls: FakeStore),
    )
    monkeypatch.setattr(
        "omni.foundation.config.database.get_memory_db_path",
        lambda: tmp_path / "memory.hippocampus.lance",
    )

    matrix = matrix_module.NeuralMatrix(dimension=2048)
    assert init_calls == []

    assert matrix.is_active is False
    assert len(init_calls) == 0

    stats = matrix.stats()
    assert stats.get("active") is True
    assert len(init_calls) == 1
