#!/usr/bin/env python3
"""Repair DSQ snapshot alignment by repacking tensors with proper padding."""

from __future__ import annotations

import argparse
import os
import struct
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import BinaryIO

ALIGNMENT_BY_DTYPE = {
    0: 4,  # F32
    1: 2,  # F16
    8: 2,  # Q8_0
    12: 2,  # Q4K
    14: 2,  # Q6K
    16: 2,  # BF16
}


@dataclass
class Header:
    version: int
    candle_version: str
    model_id: str
    backend: str
    default_dtype: int
    block_size: int
    tensors: list["TensorEntry"]


@dataclass
class TensorEntry:
    name: str
    out_dim: int
    in_dim: int
    q_dtype: int
    q_offset: int
    q_len: int
    bias_offset: int
    bias_len: int
    bias_dtype: int
    new_q_offset: int = 0
    new_bias_offset: int = 0


def _read_u32(handle: BinaryIO) -> int:
    return struct.unpack("<I", handle.read(4))[0]


def _read_u64(handle: BinaryIO) -> int:
    return struct.unpack("<Q", handle.read(8))[0]


def _read_string(handle: BinaryIO) -> str:
    length = _read_u32(handle)
    raw = handle.read(length)
    return raw.decode("utf-8", errors="replace")


def _write_u32(handle: BinaryIO, value: int) -> None:
    handle.write(struct.pack("<I", value))


def _write_u64(handle: BinaryIO, value: int) -> None:
    handle.write(struct.pack("<Q", value))


def _write_string(handle: BinaryIO, value: str) -> None:
    data = value.encode("utf-8")
    _write_u32(handle, len(data))
    handle.write(data)


def _align_up(offset: int, alignment: int) -> int:
    if alignment <= 1:
        return offset
    return (offset + alignment - 1) // alignment * alignment


def _header_size(header: Header) -> int:
    size = 0
    size += 7  # magic
    size += 4  # version
    size += 4 + len(header.candle_version.encode("utf-8"))
    size += 4 + len(header.model_id.encode("utf-8"))
    size += 4 + len(header.backend.encode("utf-8"))
    size += 4  # default_dtype
    size += 4  # block_size
    size += 4  # tensor_count
    for tensor in header.tensors:
        size += 4 + len(tensor.name.encode("utf-8"))
        size += 4 * 3  # out_dim, in_dim, q_dtype
        size += 8 * 4  # q_offset, q_len, bias_offset, bias_len
        size += 4  # bias_dtype
    return size


def _parse_header(path: Path) -> Header:
    with path.open("rb") as handle:
        magic = handle.read(7)
        if magic != b"DSQSNAP":
            raise ValueError(f"Invalid DSQ magic for {path}")
        version = _read_u32(handle)
        candle_version = _read_string(handle)
        model_id = _read_string(handle)
        backend = _read_string(handle)
        default_dtype = _read_u32(handle)
        block_size = _read_u32(handle)
        tensor_count = _read_u32(handle)
        tensors: list[TensorEntry] = []
        for _ in range(tensor_count):
            name = _read_string(handle)
            out_dim = _read_u32(handle)
            in_dim = _read_u32(handle)
            q_dtype = _read_u32(handle)
            q_offset = _read_u64(handle)
            q_len = _read_u64(handle)
            bias_offset = _read_u64(handle)
            bias_len = _read_u64(handle)
            bias_dtype = _read_u32(handle)
            tensors.append(
                TensorEntry(
                    name=name,
                    out_dim=out_dim,
                    in_dim=in_dim,
                    q_dtype=q_dtype,
                    q_offset=q_offset,
                    q_len=q_len,
                    bias_offset=bias_offset,
                    bias_len=bias_len,
                    bias_dtype=bias_dtype,
                )
            )
        return Header(
            version=version,
            candle_version=candle_version,
            model_id=model_id,
            backend=backend,
            default_dtype=default_dtype,
            block_size=block_size,
            tensors=tensors,
        )


def _compute_layout(header: Header) -> int:
    offset = _header_size(header)
    for tensor in header.tensors:
        if tensor.q_len:
            alignment = ALIGNMENT_BY_DTYPE.get(tensor.q_dtype, 1)
            offset = _align_up(offset, alignment)
            tensor.new_q_offset = offset
            offset += tensor.q_len
        else:
            tensor.new_q_offset = 0
        if tensor.bias_len:
            alignment = ALIGNMENT_BY_DTYPE.get(tensor.bias_dtype, 1)
            offset = _align_up(offset, alignment)
            tensor.new_bias_offset = offset
            offset += tensor.bias_len
        else:
            tensor.new_bias_offset = 0
    return offset


def _copy_range(source: BinaryIO, dest: BinaryIO, offset: int, length: int) -> None:
    if length == 0:
        return
    source.seek(offset)
    remaining = length
    chunk_size = 8 * 1024 * 1024
    while remaining:
        chunk = source.read(min(chunk_size, remaining))
        if not chunk:
            raise IOError("Unexpected EOF while reading DSQ payload")
        dest.write(chunk)
        remaining -= len(chunk)


def _write_header(handle: BinaryIO, header: Header) -> None:
    handle.write(b"DSQSNAP")
    _write_u32(handle, header.version)
    _write_string(handle, header.candle_version)
    _write_string(handle, header.model_id)
    _write_string(handle, header.backend)
    _write_u32(handle, header.default_dtype)
    _write_u32(handle, header.block_size)
    _write_u32(handle, len(header.tensors))
    for tensor in header.tensors:
        _write_string(handle, tensor.name)
        _write_u32(handle, tensor.out_dim)
        _write_u32(handle, tensor.in_dim)
        _write_u32(handle, tensor.q_dtype)
        _write_u64(handle, tensor.new_q_offset)
        _write_u64(handle, tensor.q_len)
        _write_u64(handle, tensor.new_bias_offset)
        _write_u64(handle, tensor.bias_len)
        _write_u32(handle, tensor.bias_dtype)


def repair_dsq(path: Path, output: Path) -> bool:
    header = _parse_header(path)
    original_header_size = _header_size(header)
    _compute_layout(header)
    new_header_size = _header_size(header)
    if original_header_size != new_header_size:
        raise ValueError("Header size mismatch after layout computation")

    changed = any(
        tensor.q_offset != tensor.new_q_offset or tensor.bias_offset != tensor.new_bias_offset
        for tensor in header.tensors
    )

    with path.open("rb") as source, output.open("wb") as dest:
        _write_header(dest, header)
        if dest.tell() != new_header_size:
            raise ValueError("Header write length mismatch")

        for tensor in header.tensors:
            if tensor.q_len:
                if dest.tell() > tensor.new_q_offset:
                    raise ValueError("Output position exceeded expected q_offset")
                dest.write(b"\x00" * (tensor.new_q_offset - dest.tell()))
                _copy_range(source, dest, tensor.q_offset, tensor.q_len)
            if tensor.bias_len:
                if dest.tell() > tensor.new_bias_offset:
                    raise ValueError("Output position exceeded expected bias_offset")
                dest.write(b"\x00" * (tensor.new_bias_offset - dest.tell()))
                _copy_range(source, dest, tensor.bias_offset, tensor.bias_len)

    return changed


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Repair DSQ snapshot alignment in-place.")
    parser.add_argument("dsq_path", help="Path to .dsq file")
    parser.add_argument(
        "--in-place",
        action="store_true",
        help="Replace the input file after repair",
    )
    parser.add_argument(
        "--output",
        default=None,
        help="Optional output path (defaults to <input>.repaired.dsq)",
    )
    return parser


def main() -> int:
    args = _build_parser().parse_args()
    dsq_path = Path(args.dsq_path).expanduser()
    if not dsq_path.is_file():
        raise SystemExit(f"DSQ file not found: {dsq_path}")

    if args.output:
        output_path = Path(args.output).expanduser()
        changed = repair_dsq(dsq_path, output_path)
        print(f"Wrote repaired DSQ to {output_path} (changed={changed})")
        return 0

    if not args.in_place:
        output_path = dsq_path.with_suffix(dsq_path.suffix + ".repaired")
        changed = repair_dsq(dsq_path, output_path)
        print(f"Wrote repaired DSQ to {output_path} (changed={changed})")
        return 0

    with tempfile.NamedTemporaryFile(
        prefix=f"{dsq_path.name}.repair.",
        suffix=".dsq",
        dir=str(dsq_path.parent),
        delete=False,
    ) as temp_handle:
        temp_path = Path(temp_handle.name)

    changed = repair_dsq(dsq_path, temp_path)
    os.replace(temp_path, dsq_path)
    print(f"Repaired DSQ in place: {dsq_path} (changed={changed})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
