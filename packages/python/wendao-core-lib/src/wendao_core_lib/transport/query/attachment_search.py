"""Typed attachment-search query helpers for Wendao Flight transport clients."""

from __future__ import annotations

from dataclasses import dataclass

from ..config import (
    ATTACHMENT_SEARCH_DEFAULT_LIMIT,
    WENDAO_ATTACHMENT_SEARCH_CASE_SENSITIVE_HEADER,
    WENDAO_ATTACHMENT_SEARCH_EXT_FILTERS_HEADER,
    WENDAO_ATTACHMENT_SEARCH_KIND_FILTERS_HEADER,
    WENDAO_SEARCH_LIMIT_HEADER,
    WENDAO_SEARCH_QUERY_HEADER,
)
from .common import WendaoFlightRouteQuery

SEARCH_ATTACHMENTS_ROUTE = "/search/attachments"
ATTACHMENT_SEARCH_NAME_COLUMN = "name"
ATTACHMENT_SEARCH_PATH_COLUMN = "path"
ATTACHMENT_SEARCH_SOURCE_ID_COLUMN = "sourceId"
ATTACHMENT_SEARCH_SOURCE_STEM_COLUMN = "sourceStem"
ATTACHMENT_SEARCH_SOURCE_TITLE_COLUMN = "sourceTitle"
ATTACHMENT_SEARCH_NAVIGATION_TARGET_JSON_COLUMN = "navigationTargetJson"
ATTACHMENT_SEARCH_SOURCE_PATH_COLUMN = "sourcePath"
ATTACHMENT_SEARCH_ATTACHMENT_ID_COLUMN = "attachmentId"
ATTACHMENT_SEARCH_ATTACHMENT_PATH_COLUMN = "attachmentPath"
ATTACHMENT_SEARCH_ATTACHMENT_NAME_COLUMN = "attachmentName"
ATTACHMENT_SEARCH_ATTACHMENT_EXT_COLUMN = "attachmentExt"
ATTACHMENT_SEARCH_KIND_COLUMN = "kind"
ATTACHMENT_SEARCH_SCORE_COLUMN = "score"
ATTACHMENT_SEARCH_VISION_SNIPPET_COLUMN = "visionSnippet"
ATTACHMENT_SEARCH_COLUMNS = (
    ATTACHMENT_SEARCH_NAME_COLUMN,
    ATTACHMENT_SEARCH_PATH_COLUMN,
    ATTACHMENT_SEARCH_SOURCE_ID_COLUMN,
    ATTACHMENT_SEARCH_SOURCE_STEM_COLUMN,
    ATTACHMENT_SEARCH_SOURCE_TITLE_COLUMN,
    ATTACHMENT_SEARCH_NAVIGATION_TARGET_JSON_COLUMN,
    ATTACHMENT_SEARCH_SOURCE_PATH_COLUMN,
    ATTACHMENT_SEARCH_ATTACHMENT_ID_COLUMN,
    ATTACHMENT_SEARCH_ATTACHMENT_PATH_COLUMN,
    ATTACHMENT_SEARCH_ATTACHMENT_NAME_COLUMN,
    ATTACHMENT_SEARCH_ATTACHMENT_EXT_COLUMN,
    ATTACHMENT_SEARCH_KIND_COLUMN,
    ATTACHMENT_SEARCH_SCORE_COLUMN,
    ATTACHMENT_SEARCH_VISION_SNIPPET_COLUMN,
)


@dataclass(frozen=True, slots=True)
class WendaoAttachmentSearchResultRow:
    """Typed row for the stable Wendao attachment-search query contract."""

    name: str
    path: str
    source_id: str
    source_stem: str
    source_title: str
    navigation_target_json: str | None
    source_path: str
    attachment_id: str
    attachment_path: str
    attachment_name: str
    attachment_ext: str
    kind: str
    score: float
    vision_snippet: str | None


@dataclass(frozen=True, slots=True)
class WendaoAttachmentSearchRequest:
    """Typed request for the stable Wendao attachment-search contract."""

    query_text: str
    limit: int = ATTACHMENT_SEARCH_DEFAULT_LIMIT
    ext_filters: tuple[str, ...] = ()
    kind_filters: tuple[str, ...] = ()
    case_sensitive: bool = False


def attachment_search_query(*, ticket: str | bytes | None = None) -> WendaoFlightRouteQuery:
    """Build the stable Wendao attachment-search query."""

    return WendaoFlightRouteQuery(route=SEARCH_ATTACHMENTS_ROUTE, ticket=ticket)


def attachment_search_request(
    query_text: str,
    *,
    limit: int = ATTACHMENT_SEARCH_DEFAULT_LIMIT,
    ext_filters: tuple[str, ...] | list[str] = (),
    kind_filters: tuple[str, ...] | list[str] = (),
    case_sensitive: bool = False,
) -> WendaoAttachmentSearchRequest:
    """Build the stable Wendao attachment-search request."""

    return WendaoAttachmentSearchRequest(
        query_text=query_text,
        limit=limit,
        ext_filters=tuple(ext_filters),
        kind_filters=tuple(kind_filters),
        case_sensitive=case_sensitive,
    )


def validate_attachment_search_table(table) -> None:
    """Validate that one Arrow table matches the stable attachment-search columns."""

    missing = [column for column in ATTACHMENT_SEARCH_COLUMNS if column not in table.column_names]
    if missing:
        raise ValueError(
            "attachment search table is missing required columns: " + ", ".join(missing)
        )


def validate_attachment_search_request(request: WendaoAttachmentSearchRequest) -> None:
    """Validate one typed attachment-search request."""

    if not request.query_text.strip():
        raise ValueError("attachment search query text must not be blank")
    if request.limit <= 0:
        raise ValueError("attachment search limit must be greater than zero")
    for ext_filter in request.ext_filters:
        if not ext_filter.strip():
            raise ValueError("attachment search extension filters must not contain blank values")
    for kind_filter in request.kind_filters:
        if not kind_filter.strip():
            raise ValueError("attachment search kind filters must not contain blank values")


def normalized_attachment_search_ext_filters(
    request: WendaoAttachmentSearchRequest,
) -> tuple[str, ...]:
    """Return sorted unique extension filters for one attachment-search request."""

    validate_attachment_search_request(request)
    return tuple(sorted({ext_filter.strip().lower() for ext_filter in request.ext_filters}))


def normalized_attachment_search_kind_filters(
    request: WendaoAttachmentSearchRequest,
) -> tuple[str, ...]:
    """Return sorted unique kind filters for one attachment-search request."""

    validate_attachment_search_request(request)
    return tuple(sorted({kind_filter.strip().lower() for kind_filter in request.kind_filters}))


def attachment_search_metadata(request: WendaoAttachmentSearchRequest) -> dict[str, str]:
    """Build Flight metadata for one typed attachment-search request."""

    validate_attachment_search_request(request)
    metadata = {
        WENDAO_SEARCH_QUERY_HEADER: request.query_text,
        WENDAO_SEARCH_LIMIT_HEADER: str(request.limit),
        WENDAO_ATTACHMENT_SEARCH_CASE_SENSITIVE_HEADER: str(request.case_sensitive).lower(),
    }
    ext_filters = normalized_attachment_search_ext_filters(request)
    if ext_filters:
        metadata[WENDAO_ATTACHMENT_SEARCH_EXT_FILTERS_HEADER] = ",".join(ext_filters)
    kind_filters = normalized_attachment_search_kind_filters(request)
    if kind_filters:
        metadata[WENDAO_ATTACHMENT_SEARCH_KIND_FILTERS_HEADER] = ",".join(kind_filters)
    return metadata


def parse_attachment_search_rows(table) -> list[WendaoAttachmentSearchResultRow]:
    """Parse one attachment-search Arrow table into typed Python rows."""

    validate_attachment_search_table(table)
    rows = table.select(ATTACHMENT_SEARCH_COLUMNS).to_pylist()
    return [
        WendaoAttachmentSearchResultRow(
            name=str(row[ATTACHMENT_SEARCH_NAME_COLUMN]),
            path=str(row[ATTACHMENT_SEARCH_PATH_COLUMN]),
            source_id=str(row[ATTACHMENT_SEARCH_SOURCE_ID_COLUMN]),
            source_stem=str(row[ATTACHMENT_SEARCH_SOURCE_STEM_COLUMN]),
            source_title=str(row[ATTACHMENT_SEARCH_SOURCE_TITLE_COLUMN]),
            navigation_target_json=(
                str(row[ATTACHMENT_SEARCH_NAVIGATION_TARGET_JSON_COLUMN])
                if row[ATTACHMENT_SEARCH_NAVIGATION_TARGET_JSON_COLUMN] is not None
                else None
            ),
            source_path=str(row[ATTACHMENT_SEARCH_SOURCE_PATH_COLUMN]),
            attachment_id=str(row[ATTACHMENT_SEARCH_ATTACHMENT_ID_COLUMN]),
            attachment_path=str(row[ATTACHMENT_SEARCH_ATTACHMENT_PATH_COLUMN]),
            attachment_name=str(row[ATTACHMENT_SEARCH_ATTACHMENT_NAME_COLUMN]),
            attachment_ext=str(row[ATTACHMENT_SEARCH_ATTACHMENT_EXT_COLUMN]),
            kind=str(row[ATTACHMENT_SEARCH_KIND_COLUMN]),
            score=float(row[ATTACHMENT_SEARCH_SCORE_COLUMN]),
            vision_snippet=(
                str(row[ATTACHMENT_SEARCH_VISION_SNIPPET_COLUMN])
                if row[ATTACHMENT_SEARCH_VISION_SNIPPET_COLUMN] is not None
                else None
            ),
        )
        for row in rows
    ]


__all__ = [
    "ATTACHMENT_SEARCH_ATTACHMENT_EXT_COLUMN",
    "ATTACHMENT_SEARCH_ATTACHMENT_ID_COLUMN",
    "ATTACHMENT_SEARCH_ATTACHMENT_NAME_COLUMN",
    "ATTACHMENT_SEARCH_ATTACHMENT_PATH_COLUMN",
    "ATTACHMENT_SEARCH_COLUMNS",
    "ATTACHMENT_SEARCH_DEFAULT_LIMIT",
    "ATTACHMENT_SEARCH_KIND_COLUMN",
    "ATTACHMENT_SEARCH_NAME_COLUMN",
    "ATTACHMENT_SEARCH_NAVIGATION_TARGET_JSON_COLUMN",
    "ATTACHMENT_SEARCH_PATH_COLUMN",
    "ATTACHMENT_SEARCH_SCORE_COLUMN",
    "ATTACHMENT_SEARCH_SOURCE_ID_COLUMN",
    "ATTACHMENT_SEARCH_SOURCE_PATH_COLUMN",
    "ATTACHMENT_SEARCH_SOURCE_STEM_COLUMN",
    "ATTACHMENT_SEARCH_SOURCE_TITLE_COLUMN",
    "ATTACHMENT_SEARCH_VISION_SNIPPET_COLUMN",
    "SEARCH_ATTACHMENTS_ROUTE",
    "WENDAO_ATTACHMENT_SEARCH_CASE_SENSITIVE_HEADER",
    "WENDAO_ATTACHMENT_SEARCH_EXT_FILTERS_HEADER",
    "WENDAO_ATTACHMENT_SEARCH_KIND_FILTERS_HEADER",
    "WENDAO_SEARCH_LIMIT_HEADER",
    "WENDAO_SEARCH_QUERY_HEADER",
    "WendaoAttachmentSearchRequest",
    "WendaoAttachmentSearchResultRow",
    "attachment_search_metadata",
    "attachment_search_query",
    "attachment_search_request",
    "normalized_attachment_search_ext_filters",
    "normalized_attachment_search_kind_filters",
    "parse_attachment_search_rows",
    "validate_attachment_search_request",
    "validate_attachment_search_table",
]
