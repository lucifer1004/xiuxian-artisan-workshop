from __future__ import annotations

import pyarrow as pa
import pytest

from xiuxian_wendao_py.transport import (
    REPO_SEARCH_BEST_SECTION_COLUMN,
    REPO_SEARCH_DEFAULT_LIMIT,
    REPO_SEARCH_COLUMNS,
    REPO_SEARCH_ROUTE,
    RERANK_EXCHANGE_ROUTE,
    RERANK_REQUEST_COLUMNS,
    RERANK_REQUEST_EMBEDDING_COLUMN,
    RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
    RERANK_RESPONSE_COLUMNS,
    WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER,
    WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER,
    WENDAO_REPO_SEARCH_LIMIT_HEADER,
    WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER,
    WENDAO_REPO_SEARCH_QUERY_HEADER,
    WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER,
    WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER,
    WendaoRepoSearchRequest,
    WendaoRepoSearchResultRow,
    WendaoRerankRequestRow,
    WendaoRerankResultRow,
    build_rerank_request_table,
    parse_rerank_response_rows,
    parse_repo_search_rows,
    normalized_repo_search_language_filters,
    normalized_repo_search_filename_filters,
    normalized_repo_search_path_prefixes,
    normalized_repo_search_tag_filters,
    normalized_repo_search_title_filters,
    repo_search_metadata,
    repo_search_query,
    repo_search_request,
    rerank_exchange_query,
    rerank_embedding_dimension,
    validate_repo_search_request,
    validate_rerank_response_table,
    validate_rerank_request_table,
    validate_repo_search_table,
)


def test_repo_search_query_uses_stable_route_and_ticket() -> None:
    query = repo_search_query()

    assert query.normalized_route() == REPO_SEARCH_ROUTE
    assert query.effective_ticket() == REPO_SEARCH_ROUTE


def test_validate_repo_search_table_rejects_missing_columns() -> None:
    table = pa.table({"doc_id": ["doc-1"], "path": ["src/lib.rs"]})

    with pytest.raises(ValueError, match="title, best_section, score, language"):
        validate_repo_search_table(table)


def test_repo_search_request_uses_stable_defaults() -> None:
    request = repo_search_request("rerank rust traits")

    assert request == WendaoRepoSearchRequest(
        query_text="rerank rust traits",
        limit=REPO_SEARCH_DEFAULT_LIMIT,
        language_filters=(),
        path_prefixes=(),
        title_filters=(),
        tag_filters=(),
        filename_filters=(),
    )


def test_repo_search_request_normalizes_language_filters() -> None:
    request = repo_search_request(
        "rerank rust traits",
        language_filters=(" rust ", "markdown", "rust"),
    )

    assert normalized_repo_search_language_filters(request) == ("markdown", "rust")


def test_repo_search_request_normalizes_path_prefixes() -> None:
    request = repo_search_request(
        "rerank rust traits",
        path_prefixes=(" src/lib", "README", "src/lib"),
    )

    assert normalized_repo_search_path_prefixes(request) == ("README", "src/lib")


def test_repo_search_request_normalizes_title_filters() -> None:
    request = repo_search_request(
        "rerank rust traits",
        title_filters=(" readme ", "overview", "readme"),
    )

    assert normalized_repo_search_title_filters(request) == ("overview", "readme")


def test_repo_search_request_normalizes_tag_filters() -> None:
    request = repo_search_request(
        "rerank rust traits",
        tag_filters=(" lang:rust ", "code", "lang:rust"),
    )

    assert normalized_repo_search_tag_filters(request) == ("code", "lang:rust")


def test_repo_search_request_normalizes_filename_filters() -> None:
    request = repo_search_request(
        "rerank rust traits",
        filename_filters=(" readme.md ", "lib.rs", "readme.md"),
    )

    assert normalized_repo_search_filename_filters(request) == ("lib.rs", "readme.md")


def test_validate_repo_search_request_rejects_blank_query_text() -> None:
    with pytest.raises(ValueError, match="query text must not be blank"):
        validate_repo_search_request(WendaoRepoSearchRequest(query_text="   "))


def test_repo_search_metadata_builds_stable_headers() -> None:
    metadata = repo_search_metadata(
        WendaoRepoSearchRequest(
            query_text="rerank rust traits",
            limit=25,
            language_filters=("markdown", "rust"),
            path_prefixes=("README", "src/"),
            title_filters=("README", "overview"),
            tag_filters=("code", "lang:rust"),
            filename_filters=("README.md", "lib.rs"),
        )
    )

    assert metadata == {
        WENDAO_REPO_SEARCH_QUERY_HEADER: "rerank rust traits",
        WENDAO_REPO_SEARCH_LIMIT_HEADER: "25",
        WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER: "markdown,rust",
        WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER: "README,src/",
        WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER: "README,overview",
        WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER: "code,lang:rust",
        WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER: "README.md,lib.rs",
    }


def test_validate_repo_search_request_rejects_blank_language_filters() -> None:
    with pytest.raises(ValueError, match="language filters must not contain blank values"):
        validate_repo_search_request(
            WendaoRepoSearchRequest(
                query_text="rerank rust traits",
                language_filters=("rust", "   "),
            )
        )


def test_validate_repo_search_request_rejects_blank_path_prefixes() -> None:
    with pytest.raises(ValueError, match="path prefixes must not contain blank values"):
        validate_repo_search_request(
            WendaoRepoSearchRequest(
                query_text="rerank rust traits",
                path_prefixes=("src/", "   "),
            )
        )


def test_validate_repo_search_request_rejects_blank_title_filters() -> None:
    with pytest.raises(ValueError, match="title filters must not contain blank values"):
        validate_repo_search_request(
            WendaoRepoSearchRequest(
                query_text="rerank rust traits",
                title_filters=("README", "   "),
            )
        )


def test_validate_repo_search_request_rejects_blank_tag_filters() -> None:
    with pytest.raises(ValueError, match="tag filters must not contain blank values"):
        validate_repo_search_request(
            WendaoRepoSearchRequest(
                query_text="rerank rust traits",
                tag_filters=("lang:rust", "   "),
            )
        )


def test_validate_repo_search_request_rejects_blank_filename_filters() -> None:
    with pytest.raises(ValueError, match="filename filters must not contain blank values"):
        validate_repo_search_request(
            WendaoRepoSearchRequest(
                query_text="rerank rust traits",
                filename_filters=("README.md", "   "),
            )
        )


def test_parse_repo_search_rows_builds_typed_rows() -> None:
    table = pa.table(
        {
            "doc_id": ["doc-1"],
            "path": ["src/lib.rs"],
            "title": ["Repo Search Result"],
            "best_section": ["12: Repo Search Result section"],
            "score": [0.91],
            "language": ["rust"],
        }
    )

    rows = parse_repo_search_rows(table)

    assert rows == [
        WendaoRepoSearchResultRow(
            doc_id="doc-1",
            path="src/lib.rs",
            title="Repo Search Result",
            best_section="12: Repo Search Result section",
            score=0.91,
            language="rust",
        )
    ]
    assert rows[0].best_section == str(table[REPO_SEARCH_BEST_SECTION_COLUMN][0].as_py())
    assert tuple(table.column_names) == REPO_SEARCH_COLUMNS


def test_rerank_exchange_query_uses_stable_route_and_ticket() -> None:
    query = rerank_exchange_query()

    assert query.normalized_route() == RERANK_EXCHANGE_ROUTE
    assert query.effective_ticket() == RERANK_EXCHANGE_ROUTE


def test_validate_rerank_request_table_rejects_missing_columns() -> None:
    table = pa.table({"doc_id": ["doc-1"]})

    with pytest.raises(ValueError, match="vector_score, embedding, query_embedding"):
        validate_rerank_request_table(table)


def test_build_rerank_request_table_builds_stable_columns() -> None:
    table = build_rerank_request_table(
        [
            WendaoRerankRequestRow(
                doc_id="doc-1",
                vector_score=0.91,
                embedding=(0.1, 0.2, 0.3),
                query_embedding=(0.4, 0.5, 0.6),
            ),
            WendaoRerankRequestRow(
                doc_id="doc-2",
                vector_score=0.72,
                embedding=(0.7, 0.8, 0.9),
                query_embedding=(1.0, 1.1, 1.2),
            ),
        ]
    )

    assert tuple(table.column_names) == RERANK_REQUEST_COLUMNS
    assert table.schema.field(RERANK_REQUEST_EMBEDDING_COLUMN).type == pa.list_(
        pa.float32(), 3
    )
    assert table.schema.field(RERANK_REQUEST_QUERY_EMBEDDING_COLUMN).type == pa.list_(
        pa.float32(), 3
    )
    assert table.to_pylist() == [
        {
            "doc_id": "doc-1",
            "vector_score": pytest.approx(0.91),
            "embedding": [pytest.approx(0.1), pytest.approx(0.2), pytest.approx(0.3)],
            "query_embedding": [
                pytest.approx(0.4),
                pytest.approx(0.5),
                pytest.approx(0.6),
            ],
        },
        {
            "doc_id": "doc-2",
            "vector_score": pytest.approx(0.72),
            "embedding": [pytest.approx(0.7), pytest.approx(0.8), pytest.approx(0.9)],
            "query_embedding": [
                pytest.approx(1.0),
                pytest.approx(1.1),
                pytest.approx(1.2),
            ],
        },
    ]


def test_validate_rerank_response_table_rejects_missing_columns() -> None:
    table = pa.table({"doc_id": ["doc-1"], "final_score": [0.97]})

    with pytest.raises(ValueError, match="rank"):
        validate_rerank_response_table(table)


def test_parse_rerank_response_rows_builds_typed_rows() -> None:
    table = pa.table(
        {
            "doc_id": ["doc-1"],
            "final_score": [0.97],
            "rank": [1],
        }
    )

    rows = parse_rerank_response_rows(table)

    assert rows == [
        WendaoRerankResultRow(
            doc_id="doc-1",
            final_score=0.97,
            rank=1,
        )
    ]
    assert tuple(table.column_names) == RERANK_RESPONSE_COLUMNS


def test_rerank_embedding_dimension_rejects_mismatched_embedding_shapes() -> None:
    rows = [
        WendaoRerankRequestRow(
            doc_id="doc-1",
            vector_score=0.91,
            embedding=(0.1, 0.2, 0.3),
            query_embedding=(0.4, 0.5, 0.6),
        ),
        WendaoRerankRequestRow(
            doc_id="doc-2",
            vector_score=0.72,
            embedding=(0.7, 0.8),
            query_embedding=(0.9, 1.0),
        ),
    ]

    with pytest.raises(ValueError, match="embedding dimensions must match across all rows"):
        rerank_embedding_dimension(rows)


def test_rerank_embedding_dimension_rejects_query_embedding_shape_drift() -> None:
    rows = [
        WendaoRerankRequestRow(
            doc_id="doc-1",
            vector_score=0.91,
            embedding=(0.1, 0.2, 0.3),
            query_embedding=(0.4, 0.5),
        ),
    ]

    with pytest.raises(
        ValueError,
        match="query embedding dimension must match embedding dimension",
    ):
        build_rerank_request_table(rows)
