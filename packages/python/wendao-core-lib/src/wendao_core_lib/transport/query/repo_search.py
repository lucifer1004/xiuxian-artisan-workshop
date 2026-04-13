"""Typed repo-search query helpers for Wendao Flight transport clients."""

from __future__ import annotations

from dataclasses import dataclass

from ..config import (
    REPO_SEARCH_DEFAULT_LIMIT,
    WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER,
    WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER,
    WENDAO_REPO_SEARCH_LIMIT_HEADER,
    WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER,
    WENDAO_REPO_SEARCH_QUERY_HEADER,
    WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER,
    WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER,
)
from .common import WendaoFlightRouteQuery

REPO_SEARCH_ROUTE = "/search/repos/main"
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


__all__ = [
    "REPO_SEARCH_BEST_SECTION_COLUMN",
    "REPO_SEARCH_COLUMNS",
    "REPO_SEARCH_DEFAULT_LIMIT",
    "REPO_SEARCH_DOC_ID_COLUMN",
    "REPO_SEARCH_HIERARCHY_COLUMN",
    "REPO_SEARCH_LANGUAGE_COLUMN",
    "REPO_SEARCH_MATCH_REASON_COLUMN",
    "REPO_SEARCH_NAVIGATION_CATEGORY_COLUMN",
    "REPO_SEARCH_NAVIGATION_LINE_COLUMN",
    "REPO_SEARCH_NAVIGATION_LINE_END_COLUMN",
    "REPO_SEARCH_NAVIGATION_PATH_COLUMN",
    "REPO_SEARCH_PATH_COLUMN",
    "REPO_SEARCH_ROUTE",
    "REPO_SEARCH_SCORE_COLUMN",
    "REPO_SEARCH_TAGS_COLUMN",
    "REPO_SEARCH_TITLE_COLUMN",
    "WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER",
    "WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER",
    "WENDAO_REPO_SEARCH_LIMIT_HEADER",
    "WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER",
    "WENDAO_REPO_SEARCH_QUERY_HEADER",
    "WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER",
    "WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER",
    "WendaoRepoSearchRequest",
    "WendaoRepoSearchResultRow",
    "normalized_repo_search_filename_filters",
    "normalized_repo_search_language_filters",
    "normalized_repo_search_path_prefixes",
    "normalized_repo_search_tag_filters",
    "normalized_repo_search_title_filters",
    "parse_repo_search_rows",
    "repo_search_metadata",
    "repo_search_query",
    "repo_search_request",
    "validate_repo_search_request",
    "validate_repo_search_table",
]
