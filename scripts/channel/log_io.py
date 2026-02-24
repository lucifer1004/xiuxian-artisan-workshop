#!/usr/bin/env python3
"""
Shared log I/O helpers for channel scripts.

Goals:
- avoid full-file log payload allocations for large runtime logs;
- keep APIs small and script-friendly (no external dependencies).
"""

from __future__ import annotations

from collections import deque
from dataclasses import dataclass
from typing import TYPE_CHECKING, Literal

if TYPE_CHECKING:
    from collections.abc import Iterator
    from pathlib import Path

DEFAULT_LOG_TAIL_BYTES = 256 * 1024
LogCursorKind = Literal["line", "offset"]


@dataclass(frozen=True)
class LogCursor:
    """Generic log cursor that can represent either line or byte offsets."""

    kind: LogCursorKind
    value: int


def iter_log_lines(path: Path, *, encoding: str = "utf-8", errors: str = "ignore") -> Iterator[str]:
    """Yield log lines without trailing newlines."""
    if not path.exists():
        return
    with path.open("r", encoding=encoding, errors=errors) as handle:
        for raw_line in handle:
            yield raw_line.rstrip("\n")


def count_log_lines(path: Path, *, encoding: str = "utf-8", errors: str = "ignore") -> int:
    """Count log lines using streaming iteration."""
    if not path.exists():
        return 0
    with path.open("r", encoding=encoding, errors=errors) as handle:
        return sum(1 for _ in handle)


def count_log_bytes(path: Path) -> int:
    """Return file size in bytes for offset-based log cursors."""
    if not path.exists():
        return 0
    return int(path.stat().st_size)


def init_log_cursor(
    path: Path,
    *,
    kind: LogCursorKind = "offset",
    encoding: str = "utf-8",
    errors: str = "ignore",
) -> LogCursor:
    """Initialize a cursor from the current file position for the requested mode."""
    if kind == "line":
        return LogCursor(kind="line", value=count_log_lines(path, encoding=encoding, errors=errors))
    return LogCursor(kind="offset", value=count_log_bytes(path))


def read_new_log_lines(
    path: Path,
    cursor: int,
    *,
    encoding: str = "utf-8",
    errors: str = "ignore",
) -> tuple[int, list[str]]:
    """
    Read lines after a line-number cursor.

    Returns `(next_cursor, lines_since_cursor)` where cursor is 0-based line count.
    """
    start_cursor = max(0, int(cursor))
    next_cursor = start_cursor
    if not path.exists():
        return next_cursor, []

    lines: list[str] = []
    with path.open("r", encoding=encoding, errors=errors) as handle:
        for index, raw_line in enumerate(handle):
            if index >= start_cursor:
                lines.append(raw_line.rstrip("\n").rstrip("\r"))
            next_cursor = index + 1
    return next_cursor, lines


def read_new_log_lines_by_offset(
    path: Path,
    offset: int,
    *,
    encoding: str = "utf-8",
    errors: str = "ignore",
) -> tuple[int, list[str]]:
    """
    Read newly appended lines after a byte-offset cursor.

    Returns `(next_offset, lines_since_offset)`. When file is truncated/rotated
    and `offset` exceeds current size, reading restarts from 0.
    """
    start_offset = max(0, int(offset))
    if not path.exists():
        return start_offset, []

    size = count_log_bytes(path)
    if start_offset > size:
        start_offset = 0

    with path.open("rb") as handle:
        if start_offset > 0:
            handle.seek(start_offset - 1)
            prev = handle.read(1)
            handle.seek(start_offset)
            # If cursor lands inside a line, skip the trailing fragment.
            skip_fragment = prev not in (b"\n", b"\r")
            if skip_fragment:
                handle.readline()
            payload = handle.read()
            # If skip consumed everything while cursor is still inside file,
            # treat it as a likely rotate/rewrite and restart from beginning.
            if skip_fragment and not payload and start_offset < size:
                handle.seek(0)
                payload = handle.read()
        else:
            handle.seek(0)
            payload = handle.read()

    lines = payload.decode(encoding, errors=errors).splitlines()
    return size, lines


def read_new_log_lines_with_cursor(
    path: Path,
    cursor: LogCursor,
    *,
    encoding: str = "utf-8",
    errors: str = "ignore",
) -> tuple[LogCursor, list[str]]:
    """Read appended log lines based on cursor mode and return the next cursor."""
    value = max(0, int(cursor.value))
    if cursor.kind == "line":
        next_value, lines = read_new_log_lines(
            path,
            value,
            encoding=encoding,
            errors=errors,
        )
        return LogCursor(kind="line", value=next_value), lines
    if cursor.kind == "offset":
        next_value, lines = read_new_log_lines_by_offset(
            path,
            value,
            encoding=encoding,
            errors=errors,
        )
        return LogCursor(kind="offset", value=next_value), lines
    raise ValueError(f"unsupported log cursor kind: {cursor.kind}")


def read_log_tail_text(
    path: Path,
    *,
    tail_bytes: int = DEFAULT_LOG_TAIL_BYTES,
    encoding: str = "utf-8",
    errors: str = "ignore",
) -> str:
    """Read only a bounded tail window from a log file."""
    if not path.exists():
        return ""

    clamped_tail = max(4 * 1024, int(tail_bytes))
    with path.open("rb") as handle:
        size = path.stat().st_size
        if size <= clamped_tail:
            payload = handle.read()
        else:
            handle.seek(size - clamped_tail)
            # Skip a potentially partial first line after tail seek.
            handle.readline()
            payload = handle.read()
    return payload.decode(encoding, errors=errors)


def read_log_tail_lines(
    path: Path,
    *,
    tail_bytes: int = DEFAULT_LOG_TAIL_BYTES,
    encoding: str = "utf-8",
    errors: str = "ignore",
) -> list[str]:
    """Read bounded log tail and split into normalized lines."""
    return read_log_tail_text(
        path,
        tail_bytes=tail_bytes,
        encoding=encoding,
        errors=errors,
    ).splitlines()


def tail_log_lines(
    path: Path,
    n: int,
    *,
    encoding: str = "utf-8",
    errors: str = "ignore",
) -> list[str]:
    """Return the last `n` lines using streaming iteration."""
    if n <= 0 or not path.exists():
        return []
    buf: deque[str] = deque(maxlen=n)
    for line in iter_log_lines(path, encoding=encoding, errors=errors):
        buf.append(line)
    return list(buf)
