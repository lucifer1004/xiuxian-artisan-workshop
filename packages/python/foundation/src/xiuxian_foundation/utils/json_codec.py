"""Common JSON codec (orjson-only, no stdlib fallback)."""

from __future__ import annotations

from typing import Any

import orjson

JSONDecodeError = orjson.JSONDecodeError


def loads(data: str | bytes | bytearray | memoryview) -> Any:
    """Parse JSON text/bytes using orjson."""
    if isinstance(data, str):
        return orjson.loads(data.encode("utf-8"))
    return orjson.loads(data)


def dumps(
    obj: Any,
    *,
    indent: int | None = None,
    ensure_ascii: bool = False,
    sort_keys: bool = False,
    separators: tuple[str, str] | None = None,
    default: Any = None,
) -> str:
    """Serialize object to JSON string using orjson.

    Supported:
    - indent: None or 2
    - ensure_ascii: False only
    - separators: None or (",", ":")
    """
    if ensure_ascii:
        raise ValueError("json_codec.dumps only supports ensure_ascii=False")

    if separators not in (None, (",", ":")):
        raise ValueError("json_codec.dumps only supports separators=None or (',', ':')")

    if indent not in (None, 2):
        raise ValueError("json_codec.dumps only supports indent=None or indent=2")

    option = 0
    if sort_keys:
        option |= orjson.OPT_SORT_KEYS
    if indent == 2:
        option |= orjson.OPT_INDENT_2

    payload = orjson.dumps(obj, option=option, default=default)
    return payload.decode("utf-8")


def dump(obj: Any, fp: Any, **kwargs: Any) -> None:
    """Write JSON to file-like object."""
    fp.write(dumps(obj, **kwargs))


def load(fp: Any) -> Any:
    """Read and parse JSON from file-like object."""
    return loads(fp.read())


__all__ = ["JSONDecodeError", "dump", "dumps", "load", "loads"]
