"""Typed Flight route-query records for xiuxian-wendao transport clients."""

from __future__ import annotations

from dataclasses import dataclass
import math

from .config import (
    WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER,
    REPO_SEARCH_DEFAULT_LIMIT,
    WENDAO_RERANK_DIMENSION_HEADER,
    WENDAO_RERANK_MIN_FINAL_SCORE_HEADER,
    WENDAO_RERANK_TOP_K_HEADER,
    WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER,
    WENDAO_REPO_SEARCH_LIMIT_HEADER,
    WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER,
    WENDAO_REPO_SEARCH_QUERY_HEADER,
    WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER,
    WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER,
)

REPO_SEARCH_ROUTE = "/search/repos/main"
RERANK_EXCHANGE_ROUTE = "/rerank/flight"
REPO_SEARCH_DOC_ID_COLUMN = "doc_id"
REPO_SEARCH_PATH_COLUMN = "path"
REPO_SEARCH_TITLE_COLUMN = "title"
REPO_SEARCH_BEST_SECTION_COLUMN = "best_section"
REPO_SEARCH_MATCH_REASON_COLUMN = "match_reason"
REPO_SEARCH_NAVIGATION_PATH_COLUMN = "navigation_path"
REPO_SEARCH_NAVIGATION_CATEGORY_COLUMN = "navigation_category"
REPO_SEARCH_NAVIGATION_LINE_COLUMN = "navigation_line"
REPO_SEARCH_NAVIGATION_LINE_END_COLUMN = "navigation_line_end"
REPO_SEARCH_HIERARCHY_COLUMN = "hierarchy"
REPO_SEARCH_TAGS_COLUMN = "tags"
REPO_SEARCH_SCORE_COLUMN = "score"
REPO_SEARCH_LANGUAGE_COLUMN = "language"
RERANK_REQUEST_DOC_ID_COLUMN = "doc_id"
RERANK_REQUEST_VECTOR_SCORE_COLUMN = "vector_score"
RERANK_REQUEST_EMBEDDING_COLUMN = "embedding"
RERANK_REQUEST_QUERY_EMBEDDING_COLUMN = "query_embedding"
RERANK_RESPONSE_DOC_ID_COLUMN = "doc_id"
RERANK_RESPONSE_VECTOR_SCORE_COLUMN = "vector_score"
RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN = "semantic_score"
RERANK_RESPONSE_FINAL_SCORE_COLUMN = "final_score"
RERANK_RESPONSE_RANK_COLUMN = "rank"
REPO_SEARCH_COLUMNS = (
    REPO_SEARCH_DOC_ID_COLUMN,
    REPO_SEARCH_PATH_COLUMN,
    REPO_SEARCH_TITLE_COLUMN,
    REPO_SEARCH_BEST_SECTION_COLUMN,
    REPO_SEARCH_MATCH_REASON_COLUMN,
    REPO_SEARCH_NAVIGATION_PATH_COLUMN,
    REPO_SEARCH_NAVIGATION_CATEGORY_COLUMN,
    REPO_SEARCH_NAVIGATION_LINE_COLUMN,
    REPO_SEARCH_NAVIGATION_LINE_END_COLUMN,
    REPO_SEARCH_HIERARCHY_COLUMN,
    REPO_SEARCH_TAGS_COLUMN,
    REPO_SEARCH_SCORE_COLUMN,
    REPO_SEARCH_LANGUAGE_COLUMN,
)
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
class WendaoFlightRouteQuery:
    """One route-backed Flight query description.

    The Rust runtime owns the actual query semantics. Python only keeps the
    route plus the effective ticket bytes needed for `get_flight_info(...)`
    and `do_get(...)`.
    """

    route: str
    ticket: str | bytes | None = None

    def normalized_route(self) -> str:
        """Return the route with a single leading slash."""
        stripped = self.route.strip()
        if not stripped or stripped == "/":
            raise ValueError(
                "Arrow Flight route query must resolve to at least one descriptor segment"
            )
        return f"/{stripped.lstrip('/')}"

    def descriptor_segments(self) -> tuple[str, ...]:
        """Return the normalized route split into descriptor segments."""
        return tuple(
            segment for segment in self.normalized_route().strip("/").split("/") if segment
        )

    def effective_ticket(self) -> str | bytes:
        """Return the explicit ticket or fall back to the normalized route."""
        return self.ticket if self.ticket is not None else self.normalized_route()


@dataclass(frozen=True, slots=True)
class WendaoRepoSearchResultRow:
    """Typed row for the stable Wendao repo-search query contract."""

    doc_id: str
    path: str
    title: str
    best_section: str
    match_reason: str
    navigation_path: str
    navigation_category: str
    navigation_line: int
    navigation_line_end: int
    hierarchy: tuple[str, ...]
    tags: tuple[str, ...]
    score: float
    language: str


@dataclass(frozen=True, slots=True)
class WendaoRepoSearchRequest:
    """Typed request for the stable Wendao repo-search contract."""

    query_text: str
    limit: int = REPO_SEARCH_DEFAULT_LIMIT
    language_filters: tuple[str, ...] = ()
    path_prefixes: tuple[str, ...] = ()
    title_filters: tuple[str, ...] = ()
    tag_filters: tuple[str, ...] = ()
    filename_filters: tuple[str, ...] = ()


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


def repo_search_query(*, ticket: str | bytes | None = None) -> WendaoFlightRouteQuery:
    """Build the stable Wendao repo-search query."""

    return WendaoFlightRouteQuery(route=REPO_SEARCH_ROUTE, ticket=ticket)


def repo_search_request(
    query_text: str,
    *,
    limit: int = REPO_SEARCH_DEFAULT_LIMIT,
    language_filters: tuple[str, ...] | list[str] = (),
    path_prefixes: tuple[str, ...] | list[str] = (),
    title_filters: tuple[str, ...] | list[str] = (),
    tag_filters: tuple[str, ...] | list[str] = (),
    filename_filters: tuple[str, ...] | list[str] = (),
) -> WendaoRepoSearchRequest:
    """Build the stable Wendao repo-search request."""

    return WendaoRepoSearchRequest(
        query_text=query_text,
        limit=limit,
        language_filters=tuple(language_filters),
        path_prefixes=tuple(path_prefixes),
        title_filters=tuple(title_filters),
        tag_filters=tuple(tag_filters),
        filename_filters=tuple(filename_filters),
    )


def rerank_exchange_query(*, ticket: str | bytes | None = None) -> WendaoFlightRouteQuery:
    """Build the stable Wendao rerank exchange query."""

    return WendaoFlightRouteQuery(route=RERANK_EXCHANGE_ROUTE, ticket=ticket)


def validate_repo_search_table(table) -> None:
    """Validate that one Arrow table matches the stable repo-search columns."""

    missing = [column for column in REPO_SEARCH_COLUMNS if column not in table.column_names]
    if missing:
        raise ValueError("repo search table is missing required columns: " + ", ".join(missing))


def validate_repo_search_request(request: WendaoRepoSearchRequest) -> None:
    """Validate one typed repo-search request."""

    if not request.query_text.strip():
        raise ValueError("repo search query text must not be blank")
    if request.limit <= 0:
        raise ValueError("repo search limit must be greater than zero")
    for language_filter in request.language_filters:
        if not language_filter.strip():
            raise ValueError("repo search language filters must not contain blank values")
    for path_prefix in request.path_prefixes:
        if not path_prefix.strip():
            raise ValueError("repo search path prefixes must not contain blank values")
    for title_filter in request.title_filters:
        if not title_filter.strip():
            raise ValueError("repo search title filters must not contain blank values")
    for tag_filter in request.tag_filters:
        if not tag_filter.strip():
            raise ValueError("repo search tag filters must not contain blank values")
    for filename_filter in request.filename_filters:
        if not filename_filter.strip():
            raise ValueError("repo search filename filters must not contain blank values")


def normalized_repo_search_language_filters(
    request: WendaoRepoSearchRequest,
) -> tuple[str, ...]:
    """Return sorted unique language filters for one repo-search request."""

    validate_repo_search_request(request)
    return tuple(sorted({language_filter.strip() for language_filter in request.language_filters}))


def normalized_repo_search_path_prefixes(
    request: WendaoRepoSearchRequest,
) -> tuple[str, ...]:
    """Return sorted unique path prefixes for one repo-search request."""

    validate_repo_search_request(request)
    return tuple(sorted({path_prefix.strip() for path_prefix in request.path_prefixes}))


def normalized_repo_search_title_filters(
    request: WendaoRepoSearchRequest,
) -> tuple[str, ...]:
    """Return sorted unique title filters for one repo-search request."""

    validate_repo_search_request(request)
    return tuple(sorted({title_filter.strip() for title_filter in request.title_filters}))


def normalized_repo_search_tag_filters(
    request: WendaoRepoSearchRequest,
) -> tuple[str, ...]:
    """Return sorted unique tag filters for one repo-search request."""

    validate_repo_search_request(request)
    return tuple(sorted({tag_filter.strip() for tag_filter in request.tag_filters}))


def normalized_repo_search_filename_filters(
    request: WendaoRepoSearchRequest,
) -> tuple[str, ...]:
    """Return sorted unique filename filters for one repo-search request."""

    validate_repo_search_request(request)
    return tuple(sorted({filename_filter.strip() for filename_filter in request.filename_filters}))


def repo_search_metadata(request: WendaoRepoSearchRequest) -> dict[str, str]:
    """Build Flight metadata for one typed repo-search request."""

    metadata = {
        WENDAO_REPO_SEARCH_QUERY_HEADER: request.query_text,
        WENDAO_REPO_SEARCH_LIMIT_HEADER: str(request.limit),
    }
    language_filters = normalized_repo_search_language_filters(request)
    if language_filters:
        metadata[WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER] = ",".join(language_filters)
    path_prefixes = normalized_repo_search_path_prefixes(request)
    if path_prefixes:
        metadata[WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER] = ",".join(path_prefixes)
    title_filters = normalized_repo_search_title_filters(request)
    if title_filters:
        metadata[WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER] = ",".join(title_filters)
    tag_filters = normalized_repo_search_tag_filters(request)
    if tag_filters:
        metadata[WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER] = ",".join(tag_filters)
    filename_filters = normalized_repo_search_filename_filters(request)
    if filename_filters:
        metadata[WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER] = ",".join(filename_filters)
    return metadata


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


def parse_repo_search_rows(table) -> list[WendaoRepoSearchResultRow]:
    """Parse one repo-search Arrow table into typed Python rows."""

    validate_repo_search_table(table)
    rows = table.select(REPO_SEARCH_COLUMNS).to_pylist()
    return [
        WendaoRepoSearchResultRow(
            doc_id=str(row[REPO_SEARCH_DOC_ID_COLUMN]),
            path=str(row[REPO_SEARCH_PATH_COLUMN]),
            title=str(row[REPO_SEARCH_TITLE_COLUMN]),
            best_section=str(row[REPO_SEARCH_BEST_SECTION_COLUMN]),
            match_reason=str(row[REPO_SEARCH_MATCH_REASON_COLUMN]),
            navigation_path=str(row[REPO_SEARCH_NAVIGATION_PATH_COLUMN]),
            navigation_category=str(row[REPO_SEARCH_NAVIGATION_CATEGORY_COLUMN]),
            navigation_line=int(row[REPO_SEARCH_NAVIGATION_LINE_COLUMN]),
            navigation_line_end=int(row[REPO_SEARCH_NAVIGATION_LINE_END_COLUMN]),
            hierarchy=tuple(str(value) for value in row[REPO_SEARCH_HIERARCHY_COLUMN]),
            tags=tuple(str(value) for value in row[REPO_SEARCH_TAGS_COLUMN]),
            score=float(row[REPO_SEARCH_SCORE_COLUMN]),
            language=str(row[REPO_SEARCH_LANGUAGE_COLUMN]),
        )
        for row in rows
    ]


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
    "REPO_SEARCH_COLUMNS",
    "REPO_SEARCH_BEST_SECTION_COLUMN",
    "REPO_SEARCH_DEFAULT_LIMIT",
    "REPO_SEARCH_DOC_ID_COLUMN",
    "REPO_SEARCH_MATCH_REASON_COLUMN",
    "REPO_SEARCH_NAVIGATION_CATEGORY_COLUMN",
    "REPO_SEARCH_HIERARCHY_COLUMN",
    "REPO_SEARCH_NAVIGATION_LINE_COLUMN",
    "REPO_SEARCH_NAVIGATION_LINE_END_COLUMN",
    "REPO_SEARCH_NAVIGATION_PATH_COLUMN",
    "REPO_SEARCH_TAGS_COLUMN",
    "WENDAO_REPO_SEARCH_LIMIT_HEADER",
    "WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER",
    "WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER",
    "WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER",
    "WENDAO_REPO_SEARCH_QUERY_HEADER",
    "WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER",
    "WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER",
    "WENDAO_RERANK_MIN_FINAL_SCORE_HEADER",
    "WENDAO_RERANK_TOP_K_HEADER",
    "REPO_SEARCH_LANGUAGE_COLUMN",
    "REPO_SEARCH_PATH_COLUMN",
    "REPO_SEARCH_ROUTE",
    "REPO_SEARCH_SCORE_COLUMN",
    "REPO_SEARCH_TITLE_COLUMN",
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
    "WendaoFlightRouteQuery",
    "WendaoRepoSearchRequest",
    "WendaoRepoSearchResultRow",
    "WendaoRerankRequestRow",
    "WendaoRerankResultRow",
    "build_rerank_request_table",
    "parse_rerank_response_rows",
    "parse_repo_search_rows",
    "repo_search_metadata",
    "repo_search_query",
    "repo_search_request",
    "rerank_exchange_query",
    "rerank_embedding_dimension",
    "rerank_request_metadata",
    "validate_repo_search_request",
    "validate_rerank_min_final_score",
    "validate_rerank_top_k",
    "normalized_repo_search_language_filters",
    "normalized_repo_search_filename_filters",
    "normalized_repo_search_path_prefixes",
    "normalized_repo_search_tag_filters",
    "normalized_repo_search_title_filters",
    "validate_rerank_response_table",
    "validate_rerank_request_table",
    "validate_repo_search_table",
]
