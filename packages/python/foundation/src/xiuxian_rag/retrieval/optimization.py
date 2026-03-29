"""Reusable optimization helpers for retrieval query and chunk workflows."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, TypeVar

if TYPE_CHECKING:
    from collections.abc import Callable

T = TypeVar("T")


@dataclass(frozen=True, slots=True)
class ChunkWindowProfile:
    """Bounds/defaults for chunk-style retrieval workflows."""

    name: str
    limit_default: int
    limit_min: int
    limit_max: int
    preview_default: int
    preview_min: int
    preview_max: int
    batch_default: int
    batch_min: int
    batch_max: int
    max_chunks_default: int
    max_chunks_min: int
    max_chunks_max: int
    snippet_default: int
    snippet_min: int
    snippet_max: int


@dataclass(frozen=True, slots=True)
class NormalizedChunkWindow:
    """Normalized parameters for chunk retrieval."""

    limit: int
    preview_limit: int
    batch_size: int
    max_chunks: int


BALANCED_PROFILE = ChunkWindowProfile(
    name="balanced",
    limit_default=5,
    limit_min=1,
    limit_max=50,
    preview_default=10,
    preview_min=1,
    preview_max=50,
    batch_default=5,
    batch_min=1,
    batch_max=20,
    max_chunks_default=15,
    max_chunks_min=1,
    max_chunks_max=100,
    snippet_default=150,
    snippet_min=50,
    snippet_max=500,
)

LATENCY_PROFILE = ChunkWindowProfile(
    name="latency",
    limit_default=5,
    limit_min=1,
    limit_max=20,
    preview_default=8,
    preview_min=1,
    preview_max=20,
    batch_default=4,
    batch_min=1,
    batch_max=10,
    max_chunks_default=12,
    max_chunks_min=1,
    max_chunks_max=30,
    snippet_default=120,
    snippet_min=50,
    snippet_max=300,
)

THROUGHPUT_PROFILE = ChunkWindowProfile(
    name="throughput",
    limit_default=10,
    limit_min=1,
    limit_max=100,
    preview_default=20,
    preview_min=1,
    preview_max=100,
    batch_default=10,
    batch_min=1,
    batch_max=30,
    max_chunks_default=30,
    max_chunks_min=1,
    max_chunks_max=200,
    snippet_default=200,
    snippet_min=50,
    snippet_max=800,
)

_PROFILES: dict[str, ChunkWindowProfile] = {
    BALANCED_PROFILE.name: BALANCED_PROFILE,
    LATENCY_PROFILE.name: LATENCY_PROFILE,
    THROUGHPUT_PROFILE.name: THROUGHPUT_PROFILE,
}

_NULLISH_STRINGS = frozenset(("", "none", "null"))
_TRUE_STRINGS = frozenset(("1", "true", "t", "yes", "y", "on", "enabled"))
_FALSE_STRINGS = frozenset(("0", "false", "f", "no", "n", "off", "disabled"))


def _is_nullish(value: Any) -> bool:
    """Return True when value should be treated as unset."""
    if value is None:
        return True
    if isinstance(value, str):
        return value.strip().lower() in _NULLISH_STRINGS
    return False


def get_chunk_window_profile(profile: str | None = None) -> ChunkWindowProfile:
    """Resolve a chunk tuning profile, defaulting to balanced."""
    key = (profile or "").strip().lower() or BALANCED_PROFILE.name
    return _PROFILES.get(key, BALANCED_PROFILE)


def clamp_int(
    value: Any,
    *,
    default: int,
    min_value: int,
    max_value: int,
) -> int:
    """Parse and clamp an integer into [min_value, max_value]."""
    try:
        parsed = int(value)
    except (TypeError, ValueError):
        parsed = default
    if parsed < min_value:
        return min_value
    if parsed > max_value:
        return max_value
    return parsed


def clamp_float(
    value: Any,
    *,
    default: float,
    min_value: float,
    max_value: float,
) -> float:
    """Parse and clamp a float into [min_value, max_value]."""
    try:
        parsed = float(value)
    except (TypeError, ValueError):
        parsed = default
    if parsed < min_value:
        return min_value
    if parsed > max_value:
        return max_value
    return parsed


def parse_int(
    value: Any,
    *,
    default: int,
    min_value: int | None = None,
    max_value: int | None = None,
) -> int:
    """Parse integer with default fallback and optional bounds."""
    try:
        parsed = int(value)
    except (TypeError, ValueError):
        parsed = default
    if min_value is not None and parsed < min_value:
        parsed = min_value
    if max_value is not None and parsed > max_value:
        parsed = max_value
    return parsed


def parse_float(
    value: Any,
    *,
    default: float,
    min_value: float | None = None,
    max_value: float | None = None,
) -> float:
    """Parse float with default fallback and optional bounds."""
    try:
        parsed = float(value)
    except (TypeError, ValueError):
        parsed = default
    if min_value is not None and parsed < min_value:
        parsed = min_value
    if max_value is not None and parsed > max_value:
        parsed = max_value
    return parsed


def parse_bool(value: Any, *, default: bool = False) -> bool:
    """Parse boolean values robustly (supports common string/int forms)."""
    if isinstance(value, bool):
        return value
    if isinstance(value, str):
        normalized = value.strip().lower()
        if normalized in _TRUE_STRINGS:
            return True
        if normalized in _FALSE_STRINGS:
            return False
        if normalized in _NULLISH_STRINGS:
            return default
    if isinstance(value, (int, float)):
        return bool(value)
    return default


def parse_optional_int(
    value: Any,
    *,
    min_value: int | None = None,
    max_value: int | None = None,
) -> int | None:
    """Parse nullable integer values; accept None/\"none\"/\"null\"/\"\" as null."""
    if _is_nullish(value):
        return None
    try:
        parsed = int(value)
    except (TypeError, ValueError):
        return None
    if min_value is not None and parsed < min_value:
        parsed = min_value
    if max_value is not None and parsed > max_value:
        parsed = max_value
    return parsed


def resolve_optional_int_from_setting(
    explicit: Any,
    *,
    setting_key: str,
    min_value: int | None = None,
    max_value: int | None = None,
) -> int | None:
    """Resolve optional int from explicit value first, then config setting."""
    from xiuxian_foundation.config.settings import get_setting

    if not _is_nullish(explicit):
        return parse_optional_int(explicit, min_value=min_value, max_value=max_value)
    return parse_optional_int(
        get_setting(setting_key),
        min_value=min_value,
        max_value=max_value,
    )


def resolve_bool_from_setting(
    *,
    setting_key: str,
    default: bool = False,
    explicit: Any = None,
) -> bool:
    """Resolve bool from explicit value first, then config setting."""
    from xiuxian_foundation.config.settings import get_setting

    if explicit is not None:
        return parse_bool(explicit, default=default)
    return parse_bool(get_setting(setting_key, default), default=default)


def resolve_int_from_setting(
    *,
    setting_key: str,
    default: int,
    min_value: int | None = None,
    max_value: int | None = None,
    explicit: Any = None,
) -> int:
    """Resolve int from explicit value first, then config setting."""
    from xiuxian_foundation.config.settings import get_setting

    if explicit is not None:
        return parse_int(explicit, default=default, min_value=min_value, max_value=max_value)
    return parse_int(
        get_setting(setting_key, default),
        default=default,
        min_value=min_value,
        max_value=max_value,
    )


def resolve_float_from_setting(
    *,
    setting_key: str,
    default: float,
    min_value: float | None = None,
    max_value: float | None = None,
    explicit: Any = None,
) -> float:
    """Resolve float from explicit value first, then config setting."""
    from xiuxian_foundation.config.settings import get_setting

    if explicit is not None:
        return parse_float(explicit, default=default, min_value=min_value, max_value=max_value)
    return parse_float(
        get_setting(setting_key, default),
        default=default,
        min_value=min_value,
        max_value=max_value,
    )


def normalize_chunk_window(
    *,
    limit: Any,
    preview_limit: Any,
    batch_size: Any,
    max_chunks: Any,
    chunked: bool,
    profile: str | None = None,
    enforce_limit_cap: bool = True,
) -> NormalizedChunkWindow:
    """Normalize chunk window parameters using the selected tuning profile."""
    tuning = get_chunk_window_profile(profile)
    normalized_limit = clamp_int(
        limit,
        default=tuning.limit_default,
        min_value=tuning.limit_min,
        max_value=tuning.limit_max,
    )
    normalized_preview_limit = clamp_int(
        preview_limit,
        default=tuning.preview_default,
        min_value=tuning.preview_min,
        max_value=tuning.preview_max,
    )
    normalized_batch_size = clamp_int(
        batch_size,
        default=tuning.batch_default,
        min_value=tuning.batch_min,
        max_value=tuning.batch_max,
    )
    normalized_max_chunks = clamp_int(
        max_chunks,
        default=tuning.max_chunks_default,
        min_value=tuning.max_chunks_min,
        max_value=tuning.max_chunks_max,
    )
    if chunked and enforce_limit_cap:
        normalized_preview_limit = min(normalized_preview_limit, normalized_limit)
        normalized_batch_size = min(normalized_batch_size, normalized_limit)
        normalized_max_chunks = min(normalized_max_chunks, normalized_limit)
    return NormalizedChunkWindow(
        limit=normalized_limit,
        preview_limit=normalized_preview_limit,
        batch_size=normalized_batch_size,
        max_chunks=normalized_max_chunks,
    )


def normalize_snippet_chars(value: Any, *, profile: str | None = None) -> int:
    """Normalize snippet size from the selected tuning profile."""
    tuning = get_chunk_window_profile(profile)
    return clamp_int(
        value,
        default=tuning.snippet_default,
        min_value=tuning.snippet_min,
        max_value=tuning.snippet_max,
    )


def normalize_min_score(value: Any, *, default: float = 0.0) -> float:
    """Normalize result score threshold into [0.0, 1.0]."""
    return clamp_float(value, default=default, min_value=0.0, max_value=1.0)


def is_low_signal_query(query: str, *, min_non_space_chars: int = 2) -> bool:
    """Detect very short compact queries where heavy graph policy is wasteful."""
    return len("".join((query or "").split())) < min_non_space_chars


def build_preview_rows(
    rows: list[dict[str, Any]],
    *,
    preview_limit: int,
    snippet_chars: int,
) -> list[dict[str, Any]]:
    """Build preview rows without mutating the source list."""
    preview_rows: list[dict[str, Any]] = []
    for row in rows[:preview_limit]:
        copied = dict(row)
        content = str(copied.get("content") or "")
        copied["content"] = f"{content[:snippet_chars]}…" if len(content) > snippet_chars else content
        copied["preview"] = True
        preview_rows.append(copied)
    return preview_rows


def split_into_batches[T](rows: list[T], *, batch_size: int) -> list[list[T]]:
    """Split rows into stable batches."""
    if batch_size <= 0:
        return [list(rows)] if rows else []
    return [rows[index : index + batch_size] for index in range(0, len(rows), batch_size)]


def compute_batch_count(total_items: Any, *, batch_size: Any) -> int:
    """Compute batch count from a total size and batch size."""
    total = parse_int(total_items, default=0, min_value=0)
    size = parse_int(batch_size, default=1, min_value=1)
    if total == 0:
        return 0
    return (total + size - 1) // size


def slice_batch[T](rows: list[T], *, batch_index: Any, batch_size: Any) -> list[T]:
    """Extract one batch by index."""
    index = parse_int(batch_index, default=0, min_value=0)
    size = parse_int(batch_size, default=1, min_value=1)
    start = index * size
    end = start + size
    return rows[start:end]


def is_markdown_index_chunk(content: str) -> bool:
    """Detect markdown index/TOC chunks that should be demoted."""
    normalized = (content or "").strip().lower()
    if not normalized:
        return False
    if normalized.startswith("# table of contents"):
        return True
    if normalized.startswith("## table of contents"):
        return True
    if normalized.startswith("- [") and normalized.count("\n- [") >= 2:
        return True
    if "| document |" in normalized and "| description |" in normalized:
        return normalized.count("| [") >= 2
    return False


def filter_ranked_chunks(
    rows: list[dict[str, Any]],
    *,
    limit: int,
    min_score: float,
    index_detector: Callable[[str], bool] | None = None,
) -> list[dict[str, Any]]:
    """Filter ranked rows with index-like rows demoted below contentful rows."""
    detector = index_detector or is_markdown_index_chunk
    eligible: list[tuple[bool, float, dict[str, Any]]] = []
    for row in rows:
        score = float(row.get("score") or 0.0)
        if score < min_score:
            continue
        content = str(row.get("content") or "")
        is_index = detector(content)
        eligible.append((is_index, -score, row))
    eligible.sort(key=lambda item: (item[0], item[1]))
    return [row for _is_index, _neg_score, row in eligible[:limit]]


__all__ = [
    "BALANCED_PROFILE",
    "LATENCY_PROFILE",
    "THROUGHPUT_PROFILE",
    "ChunkWindowProfile",
    "NormalizedChunkWindow",
    "build_preview_rows",
    "clamp_float",
    "clamp_int",
    "compute_batch_count",
    "filter_ranked_chunks",
    "get_chunk_window_profile",
    "is_low_signal_query",
    "is_markdown_index_chunk",
    "normalize_chunk_window",
    "normalize_min_score",
    "normalize_snippet_chars",
    "parse_bool",
    "parse_float",
    "parse_int",
    "parse_optional_int",
    "resolve_bool_from_setting",
    "resolve_float_from_setting",
    "resolve_int_from_setting",
    "resolve_optional_int_from_setting",
    "slice_batch",
    "split_into_batches",
]
