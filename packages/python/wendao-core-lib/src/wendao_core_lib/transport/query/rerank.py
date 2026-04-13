"""Typed rerank exchange helpers for Wendao Flight transport clients."""

from __future__ import annotations

from dataclasses import dataclass
import math

from ..config import (
    WENDAO_RERANK_DIMENSION_HEADER,
    WENDAO_RERANK_MIN_FINAL_SCORE_HEADER,
    WENDAO_RERANK_TOP_K_HEADER,
)
from .common import WendaoFlightRouteQuery

RERANK_EXCHANGE_ROUTE = "/rerank/flight"
RERANK_REQUEST_DOC_ID_COLUMN = "doc_id"
RERANK_REQUEST_VECTOR_SCORE_COLUMN = "vector_score"
RERANK_REQUEST_EMBEDDING_COLUMN = "embedding"
RERANK_REQUEST_QUERY_EMBEDDING_COLUMN = "query_embedding"
RERANK_RESPONSE_DOC_ID_COLUMN = "doc_id"
RERANK_RESPONSE_VECTOR_SCORE_COLUMN = "vector_score"
RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN = "semantic_score"
RERANK_RESPONSE_FINAL_SCORE_COLUMN = "final_score"
RERANK_RESPONSE_RANK_COLUMN = "rank"
RERANK_REQUEST_COLUMNS = (
    RERANK_REQUEST_DOC_ID_COLUMN,
    RERANK_REQUEST_VECTOR_SCORE_COLUMN,
    RERANK_REQUEST_EMBEDDING_COLUMN,
    RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
)
RERANK_RESPONSE_COLUMNS = (
    RERANK_RESPONSE_DOC_ID_COLUMN,
    RERANK_RESPONSE_VECTOR_SCORE_COLUMN,
    RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN,
    RERANK_RESPONSE_FINAL_SCORE_COLUMN,
    RERANK_RESPONSE_RANK_COLUMN,
)


@dataclass(frozen=True, slots=True)
class WendaoRerankRequestRow:
    """Typed row for the stable Wendao rerank request contract."""

    doc_id: str
    vector_score: float
    embedding: tuple[float, ...]
    query_embedding: tuple[float, ...]


@dataclass(frozen=True, slots=True)
class WendaoRerankResultRow:
    """Typed row for the stable Wendao rerank response contract."""

    doc_id: str
    vector_score: float
    semantic_score: float
    final_score: float
    rank: int


def rerank_exchange_query(*, ticket: str | bytes | None = None) -> WendaoFlightRouteQuery:
    """Build the stable Wendao rerank exchange query."""

    return WendaoFlightRouteQuery(route=RERANK_EXCHANGE_ROUTE, ticket=ticket)


def validate_rerank_request_table(table) -> None:
    """Validate that one Arrow table matches the stable rerank request columns."""

    missing = [column for column in RERANK_REQUEST_COLUMNS if column not in table.column_names]
    if missing:
        raise ValueError("rerank request table is missing required columns: " + ", ".join(missing))


def validate_rerank_response_table(table) -> None:
    """Validate that one Arrow table matches the stable rerank response columns."""

    missing = [column for column in RERANK_RESPONSE_COLUMNS if column not in table.column_names]
    if missing:
        raise ValueError("rerank response table is missing required columns: " + ", ".join(missing))


def rerank_embedding_dimension(rows: list[WendaoRerankRequestRow]) -> int:
    """Infer and validate the stable embedding dimension for one rerank request batch."""

    if not rows:
        raise ValueError("rerank request batch must contain at least one row")
    first_dimension = len(rows[0].embedding)
    if first_dimension <= 0:
        raise ValueError("rerank request embeddings must have at least one dimension")
    first_query_dimension = len(rows[0].query_embedding)
    if first_query_dimension != first_dimension:
        raise ValueError("rerank request query embedding dimension must match embedding dimension")
    for index, row in enumerate(rows[1:], start=1):
        if len(row.embedding) != first_dimension:
            raise ValueError(
                "rerank request embedding dimensions must match across all rows; "
                f"row {index} has dimension {len(row.embedding)} instead of {first_dimension}"
            )
        if len(row.query_embedding) != first_dimension:
            raise ValueError(
                "rerank request query embedding dimensions must match embedding dimension; "
                f"row {index} has dimension {len(row.query_embedding)} instead of {first_dimension}"
            )
    return first_dimension


def validate_rerank_top_k(top_k: int | None) -> int | None:
    """Validate one optional rerank response limit."""

    if top_k is None:
        return None
    if top_k <= 0:
        raise ValueError("rerank top_k must be greater than zero")
    return top_k


def validate_rerank_min_final_score(min_final_score: float | None) -> float | None:
    """Validate one optional rerank final-score threshold."""

    if min_final_score is None:
        return None
    if not math.isfinite(min_final_score):
        raise ValueError("rerank min_final_score must be finite")
    if not 0.0 <= min_final_score <= 1.0:
        raise ValueError("rerank min_final_score must stay within inclusive range [0.0, 1.0]")
    return min_final_score


def rerank_request_metadata(
    rows: list[WendaoRerankRequestRow],
    *,
    top_k: int | None = None,
    min_final_score: float | None = None,
) -> dict[str, str]:
    """Build Flight metadata for one typed rerank request."""

    metadata = {
        WENDAO_RERANK_DIMENSION_HEADER: str(rerank_embedding_dimension(rows)),
    }
    validated_top_k = validate_rerank_top_k(top_k)
    if validated_top_k is not None:
        metadata[WENDAO_RERANK_TOP_K_HEADER] = str(validated_top_k)
    validated_min_final_score = validate_rerank_min_final_score(min_final_score)
    if validated_min_final_score is not None:
        metadata[WENDAO_RERANK_MIN_FINAL_SCORE_HEADER] = str(validated_min_final_score)
    return metadata


def build_rerank_request_table(rows: list[WendaoRerankRequestRow]):
    """Build one Arrow table for the stable rerank request contract."""

    import pyarrow as pa

    embedding_dimension = rerank_embedding_dimension(rows)
    table = pa.table(
        {
            RERANK_REQUEST_DOC_ID_COLUMN: pa.array(
                [row.doc_id for row in rows],
                type=pa.string(),
            ),
            RERANK_REQUEST_VECTOR_SCORE_COLUMN: pa.array(
                [row.vector_score for row in rows],
                type=pa.float32(),
            ),
            RERANK_REQUEST_EMBEDDING_COLUMN: pa.array(
                [list(row.embedding) for row in rows],
                type=pa.list_(pa.float32(), embedding_dimension),
            ),
            RERANK_REQUEST_QUERY_EMBEDDING_COLUMN: pa.array(
                [list(row.query_embedding) for row in rows],
                type=pa.list_(pa.float32(), embedding_dimension),
            ),
        }
    )
    validate_rerank_request_table(table)
    return table


def parse_rerank_response_rows(table) -> list[WendaoRerankResultRow]:
    """Parse one rerank response Arrow table into typed Python rows."""

    validate_rerank_response_table(table)
    rows = table.select(RERANK_RESPONSE_COLUMNS).to_pylist()
    return [
        WendaoRerankResultRow(
            doc_id=str(row[RERANK_RESPONSE_DOC_ID_COLUMN]),
            vector_score=float(row[RERANK_RESPONSE_VECTOR_SCORE_COLUMN]),
            semantic_score=float(row[RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN]),
            final_score=float(row[RERANK_RESPONSE_FINAL_SCORE_COLUMN]),
            rank=int(row[RERANK_RESPONSE_RANK_COLUMN]),
        )
        for row in rows
    ]


__all__ = [
    "RERANK_EXCHANGE_ROUTE",
    "RERANK_REQUEST_COLUMNS",
    "RERANK_REQUEST_DOC_ID_COLUMN",
    "RERANK_REQUEST_EMBEDDING_COLUMN",
    "RERANK_REQUEST_QUERY_EMBEDDING_COLUMN",
    "RERANK_REQUEST_VECTOR_SCORE_COLUMN",
    "RERANK_RESPONSE_COLUMNS",
    "RERANK_RESPONSE_DOC_ID_COLUMN",
    "RERANK_RESPONSE_FINAL_SCORE_COLUMN",
    "RERANK_RESPONSE_RANK_COLUMN",
    "RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN",
    "RERANK_RESPONSE_VECTOR_SCORE_COLUMN",
    "WENDAO_RERANK_DIMENSION_HEADER",
    "WENDAO_RERANK_MIN_FINAL_SCORE_HEADER",
    "WENDAO_RERANK_TOP_K_HEADER",
    "WendaoRerankRequestRow",
    "WendaoRerankResultRow",
    "build_rerank_request_table",
    "parse_rerank_response_rows",
    "rerank_embedding_dimension",
    "rerank_exchange_query",
    "rerank_request_metadata",
    "validate_rerank_min_final_score",
    "validate_rerank_request_table",
    "validate_rerank_response_table",
    "validate_rerank_top_k",
]
