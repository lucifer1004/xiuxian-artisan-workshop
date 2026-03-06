#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest
from log_io import (
    LogCursor,
    count_log_bytes,
    count_log_lines,
    init_log_cursor,
    iter_log_lines,
    read_log_tail_lines,
    read_log_tail_text,
    read_new_log_lines,
    read_new_log_lines_by_offset,
    read_new_log_lines_with_cursor,
    tail_log_lines,
)


def _load_channel_script(script_name: str):
    script_path = Path(__file__).resolve().with_name(script_name)
    module_name = f"test_log_io_{script_path.stem.replace('-', '_')}"
    spec = importlib.util.spec_from_file_location(module_name, script_path)
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


@pytest.mark.parametrize(
    "script_name",
    [
        "agent_channel_blackbox.py",
        "test_xiuxian_daochang_concurrent_sessions.py",
        "test_xiuxian_daochang_dedup_events.py",
        "test_xiuxian_daochang_memory_benchmark.py",
    ],
)
def test_probe_wrappers_count_lines_use_offset_cursor(monkeypatch, script_name: str) -> None:
    module = _load_channel_script(script_name)
    observed: dict[str, str] = {}

    def _fake_init(_path: object, kind: str) -> object:
        observed["kind"] = kind
        return module._SharedLogCursor(kind="offset", value=97)

    monkeypatch.setattr(module, "_shared_init_log_cursor", _fake_init)
    assert module.count_lines(Path("dummy.log")) == 97
    assert observed["kind"] == "offset"


@pytest.mark.parametrize(
    "script_name",
    [
        "agent_channel_blackbox.py",
        "test_xiuxian_daochang_concurrent_sessions.py",
        "test_xiuxian_daochang_dedup_events.py",
        "test_xiuxian_daochang_memory_benchmark.py",
    ],
)
def test_probe_wrappers_read_new_lines_use_offset_cursor(monkeypatch, script_name: str) -> None:
    module = _load_channel_script(script_name)
    observed: dict[str, object] = {}

    def _fake_read_new(_path: object, cursor: object) -> tuple[object, list[str]]:
        observed["cursor"] = cursor
        return module._SharedLogCursor(kind="offset", value=123), ["line-x", "line-y"]

    monkeypatch.setattr(module, "_shared_read_new_log_lines_with_cursor", _fake_read_new)
    cursor, lines = module.read_new_lines(Path("dummy.log"), 19)
    assert cursor == 123
    assert lines == ["line-x", "line-y"]
    assert observed["cursor"] == module._SharedLogCursor(kind="offset", value=19)


def test_iter_log_lines_yields_all_lines(tmp_path) -> None:
    log_file = tmp_path / "runtime.log"
    log_file.write_text("a\nb\nc\n", encoding="utf-8")

    assert list(iter_log_lines(log_file)) == ["a", "b", "c"]


def test_count_log_lines_streaming(tmp_path) -> None:
    log_file = tmp_path / "runtime.log"
    log_file.write_text("a\nb\nc\n", encoding="utf-8")

    assert count_log_lines(log_file) == 3
    assert count_log_lines(tmp_path / "missing.log") == 0


def test_count_log_bytes_matches_stat_size(tmp_path) -> None:
    log_file = tmp_path / "runtime.log"
    log_file.write_bytes(b"abc\n123\n")

    assert count_log_bytes(log_file) == 8
    assert count_log_bytes(tmp_path / "missing.log") == 0


def test_init_log_cursor_supports_line_and_offset_modes(tmp_path) -> None:
    log_file = tmp_path / "runtime.log"
    log_file.write_text("a\nb\n", encoding="utf-8")

    line_cursor = init_log_cursor(log_file, kind="line")
    offset_cursor = init_log_cursor(log_file, kind="offset")
    assert line_cursor == LogCursor(kind="line", value=2)
    assert offset_cursor == LogCursor(kind="offset", value=count_log_bytes(log_file))


def test_read_new_log_lines_cursor_progression(tmp_path) -> None:
    log_file = tmp_path / "runtime.log"
    log_file.write_text("a\nb\nc\n", encoding="utf-8")

    cursor, lines = read_new_log_lines(log_file, 0)
    assert cursor == 3
    assert lines == ["a", "b", "c"]

    cursor, lines = read_new_log_lines(log_file, cursor)
    assert cursor == 3
    assert lines == []

    cursor, lines = read_new_log_lines(log_file, 2)
    assert cursor == 3
    assert lines == ["c"]

    cursor, lines = read_new_log_lines(tmp_path / "missing.log", 8)
    assert cursor == 8
    assert lines == []


def test_read_new_log_lines_by_offset_progression(tmp_path) -> None:
    log_file = tmp_path / "runtime.log"
    log_file.write_text("a\nb\n", encoding="utf-8")

    offset, lines = read_new_log_lines_by_offset(log_file, 0)
    assert lines == ["a", "b"]

    with log_file.open("a", encoding="utf-8") as handle:
        handle.write("c\n")

    next_offset, next_lines = read_new_log_lines_by_offset(log_file, offset)
    assert next_offset > offset
    assert next_lines == ["c"]


def test_read_new_log_lines_by_offset_handles_truncate(tmp_path) -> None:
    log_file = tmp_path / "runtime.log"
    log_file.write_text("first\nsecond\n", encoding="utf-8")
    offset, _ = read_new_log_lines_by_offset(log_file, 0)

    log_file.write_text("after-truncate\n", encoding="utf-8")
    next_offset, lines = read_new_log_lines_by_offset(log_file, offset)
    assert next_offset == count_log_bytes(log_file)
    assert lines == ["after-truncate"]


def test_read_new_log_lines_by_offset_resets_when_offset_exceeds_size(tmp_path) -> None:
    log_file = tmp_path / "runtime.log"
    log_file.write_text("a\nb\n", encoding="utf-8")

    next_offset, lines = read_new_log_lines_by_offset(log_file, 99_999)
    assert next_offset == count_log_bytes(log_file)
    assert lines == ["a", "b"]


def test_read_new_log_lines_by_offset_skips_partial_line_fragment(tmp_path) -> None:
    log_file = tmp_path / "runtime.log"
    log_file.write_text("alpha\nbeta\ngamma\n", encoding="utf-8")
    mid_beta_offset = len(b"alpha\nbe")

    _, lines = read_new_log_lines_by_offset(log_file, mid_beta_offset)
    assert lines == ["gamma"]


def test_read_new_log_lines_with_cursor_dispatches_by_kind(tmp_path) -> None:
    log_file = tmp_path / "runtime.log"
    log_file.write_text("a\nb\n", encoding="utf-8")

    offset_cursor = init_log_cursor(log_file, kind="offset")
    next_line_cursor, line_values = read_new_log_lines_with_cursor(
        log_file,
        LogCursor(kind="line", value=0),
    )
    assert next_line_cursor == LogCursor(kind="line", value=2)
    assert line_values == ["a", "b"]

    with log_file.open("a", encoding="utf-8") as handle:
        handle.write("c\n")

    next_offset_cursor, offset_values = read_new_log_lines_with_cursor(
        log_file,
        offset_cursor,
    )
    assert next_offset_cursor.kind == "offset"
    assert offset_values == ["c"]


def test_read_log_tail_text_handles_large_prefix(tmp_path) -> None:
    log_file = tmp_path / "runtime.log"
    with log_file.open("wb") as handle:
        handle.write(b"X" * 320_000)
        handle.write(b"\n")
        handle.write(b"tail-a\ntail-b\n")

    tail = read_log_tail_text(log_file, tail_bytes=32 * 1024)
    assert "tail-a" in tail
    assert "tail-b" in tail


def test_read_log_tail_lines_returns_recent_lines(tmp_path) -> None:
    log_file = tmp_path / "runtime.log"
    with log_file.open("wb") as handle:
        handle.write(b"Y" * 320_000)
        handle.write(b"\n")
        handle.write(b"one\ntwo\nthree\n")

    tail_lines = read_log_tail_lines(log_file, tail_bytes=32 * 1024)
    assert tail_lines[-3:] == ["one", "two", "three"]


def test_tail_log_lines_returns_last_n_streaming(tmp_path) -> None:
    log_file = tmp_path / "runtime.log"
    with log_file.open("wb") as handle:
        handle.write(b"Z" * 300_000)
        handle.write(b"\n")
        handle.write(b"l1\nl2\nl3\nl4\n")

    assert tail_log_lines(log_file, 2) == ["l3", "l4"]
