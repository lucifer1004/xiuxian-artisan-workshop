"""Local Arrow IPC helpers for table-health contract tests."""

from __future__ import annotations

import io

import pyarrow as pa
import pyarrow.ipc as ipc

TABLE_HEALTH_IPC_COLUMNS = (
    "row_count",
    "fragment_count",
    "fragmentation_ratio",
    "index_names",
    "index_types",
    "recommendations",
)


def table_health_ipc_schema() -> pa.Schema:
    return pa.schema(
        [
            ("row_count", pa.uint32()),
            ("fragment_count", pa.uint64()),
            ("fragmentation_ratio", pa.float64()),
            ("index_names", pa.list_(pa.string())),
            ("index_types", pa.list_(pa.string())),
            ("recommendations", pa.list_(pa.string())),
        ]
    )


def make_table_health_ipc_bytes(
    *,
    row_count: int = 100,
    fragment_count: int = 5,
    fragmentation_ratio: float = 0.05,
    index_names: list[str] | None = None,
    index_types: list[str] | None = None,
    recommendations: list[str] | None = None,
) -> bytes:
    if index_names is None:
        index_names = ["vector", "content_fts"]
    if index_types is None:
        index_types = ["IVF_FLAT", "Inverted"]
    if recommendations is None:
        recommendations = ["run_compaction", "none"]

    schema = table_health_ipc_schema()
    table = pa.table(
        {
            "row_count": pa.array([row_count], type=pa.uint32()),
            "fragment_count": pa.array([fragment_count], type=pa.uint64()),
            "fragmentation_ratio": pa.array([fragmentation_ratio], type=pa.float64()),
            "index_names": pa.array([index_names], type=pa.list_(pa.string())),
            "index_types": pa.array([index_types], type=pa.list_(pa.string())),
            "recommendations": pa.array([recommendations], type=pa.list_(pa.string())),
        },
        schema=schema,
    )
    buf = io.BytesIO()
    with ipc.new_stream(buf, schema) as writer:
        writer.write_table(table)
    return buf.getvalue()


def decode_table_health_ipc_bytes(data: bytes) -> pa.Table:
    return ipc.open_stream(io.BytesIO(data)).read_all()


def assert_table_health_ipc_table(table: pa.Table) -> None:
    expected = set(TABLE_HEALTH_IPC_COLUMNS)
    actual = set(table.column_names)
    assert expected.issubset(actual), f"Table should have columns {expected}, got {list(actual)}"
    assert table.num_rows == 1, "Table health IPC should have one row"
