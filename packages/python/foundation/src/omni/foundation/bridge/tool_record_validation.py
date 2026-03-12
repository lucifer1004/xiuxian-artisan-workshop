"""Tool Record Validation - Strict contract for Rust → Python data flow.

Rust is the single source of truth; Python receives and validates.
No inference in Python - invalid data fails fast with clear errors.

Contract (Rust list_all_tools output after flatten):
- id: str, non-empty
- skill_name: str, non-empty (Rust guarantees via infer_skill_tool_from_id)
- tool_name: str, non-empty
- content: str (optional)
- file_path: str | None
- category: str | None
- description: str | None
"""

from __future__ import annotations


class ToolRecordValidationError(ValueError):
    """Raised when a tool record fails validation (Rust contract broken)."""

    def __init__(self, message: str, record: dict | None = None, index: int | None = None):
        super().__init__(message)
        self.record = record
        self.index = index


# Required keys - must be non-null and non-empty
_REQUIRED_KEYS = frozenset({"id", "skill_name", "tool_name"})


def validate_tool_record(record: dict, *, index: int | None = None) -> None:
    """Validate a single tool record. Raises ToolRecordValidationError if invalid.

    Args:
        record: Flattened tool dict from Rust list_all_tools.
        index: Optional 0-based index for error context.

    Raises:
        ToolRecordValidationError: When required fields are missing or empty.
    """
    if not isinstance(record, dict):
        raise ToolRecordValidationError(
            f"Tool record must be dict, got {type(record).__name__}",
            record=record,
            index=index,
        )
    missing = _REQUIRED_KEYS - set(record.keys())
    if missing:
        raise ToolRecordValidationError(
            f"Missing required keys: {sorted(missing)}. "
            f"Rust list_all_tools must populate skill_name, tool_name.",
            record=record,
            index=index,
        )
    for key in _REQUIRED_KEYS:
        val = record.get(key)
        if val is None or (isinstance(val, str) and not val.strip()):
            raise ToolRecordValidationError(
                f"Required key '{key}' must be non-null and non-empty. "
                f"Rust contract guarantees this via infer_skill_tool_from_id.",
                record=record,
                index=index,
            )


def validate_tool_records(records: list[dict]) -> None:
    """Validate a list of tool records. Fails on first invalid record.

    Args:
        records: List of flattened tool dicts from Rust list_all_tools.

    Raises:
        ToolRecordValidationError: When any record is invalid.
    """
    for i, rec in enumerate(records):
        validate_tool_record(rec, index=i)
