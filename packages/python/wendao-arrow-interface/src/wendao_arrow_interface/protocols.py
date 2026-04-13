"""Public Arrow-first parser and analyzer protocols for Wendao consumers."""

from __future__ import annotations

from typing import Protocol, TypeVar
import pyarrow as pa

ParsedT = TypeVar("ParsedT")
AnalyzedT = TypeVar("AnalyzedT")


class ArrowTableParser(Protocol[ParsedT]):
    """Protocol for parsers that consume one Arrow table."""

    def parse_table(self, table: pa.Table) -> ParsedT: ...


class RowsAnalyzer(Protocol[AnalyzedT]):
    """Protocol for analyzers that consume one list of row dictionaries."""

    def analyze_rows(self, rows: list[dict[str, object]]) -> AnalyzedT: ...


class ArrowTableAnalyzer(Protocol[AnalyzedT]):
    """Protocol for analyzers that consume one Arrow table."""

    def analyze_table(self, table: pa.Table) -> AnalyzedT: ...


__all__ = [
    "AnalyzedT",
    "ArrowTableAnalyzer",
    "ArrowTableParser",
    "ParsedT",
    "RowsAnalyzer",
]
