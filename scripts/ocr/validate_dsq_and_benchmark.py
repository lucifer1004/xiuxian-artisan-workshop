#!/usr/bin/env python3
"""Validate DeepSeek DSQ snapshots and benchmark OCR HTTP latency.

This script is intentionally dependency-free (stdlib only) so it can be run
with the repo's default Python runtime.
"""

from __future__ import annotations

import argparse
import base64
import json
import mmap
import statistics
import string
import struct
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, List, Optional


DSQ_MAGIC = b"DSQSNAP"
DSQ_VERSION = 1


class DsqError(RuntimeError):
    """Raised when DSQ validation fails."""


@dataclass(frozen=True)
class DsqHeader:
    version: int
    candle_version: str
    model_id: str
    backend: str
    default_qdtype: str
    block_size: int
    tensor_count: int


@dataclass(frozen=True)
class DsqRecord:
    name: str
    out_dim: int
    in_dim: int
    q_dtype: str
    q_offset: int
    q_len: int
    bias_offset: Optional[int]
    bias_len: Optional[int]
    bias_dtype: Optional[str]


DTYPE_CODE_TO_NAME = {
    8: "Q8_0",
    12: "Q4_K",
    14: "Q6_K",
    1: "F16",
    16: "BF16",
    0: "F32",
}

BIAS_DTYPE_CODE_TO_NAME = {
    0: "U8",
    1: "U32",
    2: "I64",
    3: "F16",
    4: "F32",
    5: "F64",
    6: "BF16",
}


def dtype_block_size(dtype: str) -> Optional[int]:
    return {"Q8_0": 32, "Q4_K": 256, "Q6_K": 256}.get(dtype)


def dtype_elem_size(dtype: str) -> Optional[int]:
    return {"F16": 2, "BF16": 2, "F32": 4}.get(dtype)


def required_alignment(dtype: str) -> int:
    if dtype == "F32":
        return 4
    return 2


class Cursor:
    def __init__(self, data: memoryview) -> None:
        self._data = data
        self._pos = 0

    @property
    def position(self) -> int:
        return self._pos

    def read_u32(self) -> int:
        value = struct.unpack_from("<I", self._data, self._pos)[0]
        self._pos += 4
        return value

    def read_u64(self) -> int:
        value = struct.unpack_from("<Q", self._data, self._pos)[0]
        self._pos += 8
        return value

    def read_exact(self, size: int) -> bytes:
        if self._pos + size > len(self._data):
            raise DsqError("DSQ file is truncated while reading metadata")
        value = self._data[self._pos : self._pos + size]
        self._pos += size
        return bytes(value)

    def read_string(self) -> str:
        length = self.read_u32()
        raw = self.read_exact(length)
        try:
            return raw.decode("utf-8")
        except UnicodeDecodeError as exc:
            raise DsqError(f"Invalid UTF-8 string in DSQ metadata: {exc}") from exc


def parse_dsq(path: Path) -> tuple[DsqHeader, List[DsqRecord], int, int]:
    if not path.exists():
        raise DsqError(f"DSQ path does not exist: {path}")
    if not path.is_file():
        raise DsqError(f"DSQ path is not a file: {path}")

    with path.open("rb") as handle:
        data = mmap.mmap(handle.fileno(), 0, access=mmap.ACCESS_READ)
        cursor = Cursor(data)

        magic = cursor.read_exact(len(DSQ_MAGIC))
        if magic != DSQ_MAGIC:
            raise DsqError(f"Invalid DSQ magic: {magic!r}")

        version = cursor.read_u32()
        if version != DSQ_VERSION:
            raise DsqError(f"Unsupported DSQ version {version}; expected {DSQ_VERSION}")

        candle_version = cursor.read_string()
        model_id = cursor.read_string()
        backend = cursor.read_string()
        default_dtype_code = cursor.read_u32()
        default_qdtype = dtype_name_from_code(default_dtype_code)
        block_size = cursor.read_u32()
        tensor_count = cursor.read_u32()

        records: List[DsqRecord] = []
        for _ in range(tensor_count):
            name = cursor.read_string()
            out_dim = cursor.read_u32()
            in_dim = cursor.read_u32()
            q_dtype_code = cursor.read_u32()
            q_dtype = dtype_name_from_code(q_dtype_code)
            q_offset = cursor.read_u64()
            q_len = cursor.read_u64()
            bias_offset_raw = cursor.read_u64()
            bias_len_raw = cursor.read_u64()
            bias_dtype_raw = cursor.read_u32()

            if bias_len_raw == 0:
                bias_offset = None
                bias_len = None
                bias_dtype = None
            else:
                bias_offset = bias_offset_raw
                bias_len = bias_len_raw
                bias_dtype = bias_dtype_name_from_code(bias_dtype_raw)

            records.append(
                DsqRecord(
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

        metadata_len = cursor.position
        total_len = len(data)
        data.close()

    header = DsqHeader(
        version=version,
        candle_version=candle_version,
        model_id=model_id,
        backend=backend,
        default_qdtype=default_qdtype,
        block_size=block_size,
        tensor_count=tensor_count,
    )
    return header, records, metadata_len, total_len


def dtype_name_from_code(code: int) -> str:
    if code not in DTYPE_CODE_TO_NAME:
        raise DsqError(f"Unsupported tensor dtype code {code}")
    return DTYPE_CODE_TO_NAME[code]


def bias_dtype_name_from_code(code: int) -> str:
    if code not in BIAS_DTYPE_CODE_TO_NAME:
        raise DsqError(f"Unsupported bias dtype code {code}")
    return BIAS_DTYPE_CODE_TO_NAME[code]


def validate_dsq(path: Path) -> None:
    start = time.perf_counter()
    header, records, metadata_len, total_len = parse_dsq(path)
    elapsed_ms = (time.perf_counter() - start) * 1000

    errors: List[str] = []
    alignment_issues: List[str] = []

    expected_block = dtype_block_size(header.default_qdtype)
    if expected_block is None:
        errors.append(f"Snapshot dtype {header.default_qdtype} is not supported")
    elif header.block_size != expected_block:
        errors.append(
            f"Snapshot block size mismatch: expected {expected_block}, got {header.block_size}"
        )

    seen_names: set[str] = set()
    for record in records:
        if record.name in seen_names:
            errors.append(f"Duplicate tensor record: {record.name}")
        seen_names.add(record.name)

        if record.q_len == 0:
            errors.append(f"Tensor `{record.name}` has empty quantized payload")
        if record.q_offset < metadata_len:
            errors.append(
                "Tensor `{}` q_offset {} overlaps metadata region ({} bytes)".format(
                    record.name, record.q_offset, metadata_len
                )
            )
        if record.q_offset + record.q_len > total_len:
            errors.append(
                "Tensor `{}` quantized slice exceeds file size ({} + {} > {})".format(
                    record.name, record.q_offset, record.q_len, total_len
                )
            )

        if record.bias_offset is None:
            if record.bias_len is not None or record.bias_dtype is not None:
                errors.append(f"Tensor `{record.name}` bias metadata is inconsistent")
        else:
            if record.bias_len is None or record.bias_dtype is None:
                errors.append(f"Tensor `{record.name}` bias metadata is incomplete")
            else:
                if record.bias_offset + record.bias_len > total_len:
                    errors.append(
                        "Tensor `{}` bias slice exceeds file size ({} + {} > {})".format(
                            record.name,
                            record.bias_offset,
                            record.bias_len,
                            total_len,
                        )
                    )

        q_block = dtype_block_size(record.q_dtype)
        if q_block is not None:
            if record.in_dim % q_block != 0:
                errors.append(
                    "Tensor `{}` in_dim {} not divisible by block size {} ({})".format(
                        record.name, record.in_dim, q_block, record.q_dtype
                    )
                )
        else:
            elem_size = dtype_elem_size(record.q_dtype)
            if elem_size is None:
                errors.append(f"Tensor `{record.name}` uses unsupported dtype {record.q_dtype}")
            else:
                expected_len = record.out_dim * record.in_dim * elem_size
                if record.q_len != expected_len:
                    errors.append(
                        "Tensor `{}` q_len {} does not match expected {} bytes for {}".format(
                            record.name, record.q_len, expected_len, record.q_dtype
                        )
                    )

        alignment = required_alignment(record.q_dtype)
        if record.q_offset % alignment != 0:
            alignment_issues.append(
                "{}: q_offset={} dtype={} (requires {}-byte alignment)".format(
                    record.name, record.q_offset, record.q_dtype, alignment
                )
            )

    print("DSQ validation report")
    print("- Path:", path)
    print("- Parsed in: {:.2f} ms".format(elapsed_ms))
    print(
        "- Header: dtype={}, block_size={}, tensors={}".format(
            header.default_qdtype, header.block_size, header.tensor_count
        )
    )
    print("- Model: {} ({})".format(header.model_id, header.backend))
    print("- Candle: {}".format(header.candle_version))

    if errors:
        print("\nValidation errors:")
        for error in errors:
            print("  -", error)

    if alignment_issues:
        print("\nAlignment errors:")
        for issue in alignment_issues[:20]:
            print("  -", issue)
        if len(alignment_issues) > 20:
            print(f"  ... {len(alignment_issues) - 20} more")

    if errors or alignment_issues:
        raise DsqError("DSQ validation failed")

    print("\nStatus: PASS (snapshot is compatible with q_offset alignment rules)")


def sniff_image_media_type(binary: bytes) -> Optional[str]:
    if binary.startswith(b"\x89PNG\r\n\x1a\n"):
        return "image/png"
    if binary.startswith(b"\xff\xd8"):
        return "image/jpeg"
    if binary.startswith(b"GIF87a") or binary.startswith(b"GIF89a"):
        return "image/gif"
    if binary.startswith(b"RIFF") and binary[8:12] == b"WEBP":
        return "image/webp"
    return None


def build_payload(
    template: str,
    image_path: Path,
    media_type: str,
    base64_data: str,
    data_uri: str,
) -> dict:
    substitutions = {
        "image_path": str(image_path),
        "filename": image_path.name,
        "mime_type": media_type,
        "base64": base64_data,
        "data_uri": data_uri,
    }
    rendered = string.Template(template).safe_substitute(substitutions)
    try:
        return json.loads(rendered)
    except json.JSONDecodeError as exc:
        raise RuntimeError(
            "Payload template did not render valid JSON. "
            "Provide a valid template string or --payload-template-file."
        ) from exc


def send_request(url: str, payload: dict, timeout: float) -> tuple[int, str]:
    body = json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(
        url,
        data=body,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=timeout) as resp:
        status = resp.getcode()
        text = resp.read().decode("utf-8", errors="replace")
        return status, text


def run_benchmark(
    url: str,
    image_path: Path,
    payload_template: str,
    requests: int,
    warmup: int,
    timeout: float,
    print_response: bool,
) -> None:
    if requests <= 0:
        raise RuntimeError("--requests must be greater than zero")
    if warmup < 0:
        raise RuntimeError("--warmup cannot be negative")

    image_bytes = image_path.read_bytes()
    media_type = sniff_image_media_type(image_bytes) or "image/png"
    base64_data = base64.b64encode(image_bytes).decode("ascii")
    data_uri = f"data:{media_type};base64,{base64_data}"
    payload = build_payload(payload_template, image_path, media_type, base64_data, data_uri)

    print("Benchmark payload preview (keys only):")
    print("- URL:", url)
    print("- Image:", image_path)
    print("- Media type:", media_type)
    print("- Payload keys:", ", ".join(sorted(payload.keys())))

    for idx in range(warmup):
        try:
            send_request(url, payload, timeout)
        except Exception as exc:
            raise RuntimeError(f"Warmup request {idx + 1} failed: {exc}") from exc

    durations: List[float] = []
    failures: List[str] = []
    sample_response: Optional[str] = None

    for idx in range(requests):
        start = time.perf_counter()
        try:
            status, text = send_request(url, payload, timeout)
            elapsed = (time.perf_counter() - start) * 1000
            durations.append(elapsed)
            if sample_response is None:
                sample_response = text
            if status < 200 or status >= 300:
                failures.append(f"request {idx + 1}: HTTP {status}")
        except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError) as exc:
            failures.append(f"request {idx + 1}: {exc}")
        except Exception as exc:
            failures.append(f"request {idx + 1}: {exc}")

    if not durations:
        raise RuntimeError("No successful requests completed")

    durations_sorted = sorted(durations)
    avg = statistics.mean(durations_sorted)
    p50 = percentile(durations_sorted, 50)
    p90 = percentile(durations_sorted, 90)
    p95 = percentile(durations_sorted, 95)
    p99 = percentile(durations_sorted, 99)

    print("\nBenchmark results (milliseconds)")
    print(f"- Requests: {requests}")
    print(f"- Success: {len(durations_sorted)}")
    if failures:
        print(f"- Failures: {len(failures)}")
        for failure in failures[:10]:
            print("  -", failure)
        if len(failures) > 10:
            print(f"  ... {len(failures) - 10} more")
    print(f"- Min: {durations_sorted[0]:.2f}")
    print(f"- Avg: {avg:.2f}")
    print(f"- P50: {p50:.2f}")
    print(f"- P90: {p90:.2f}")
    print(f"- P95: {p95:.2f}")
    print(f"- P99: {p99:.2f}")
    print(f"- Max: {durations_sorted[-1]:.2f}")

    if print_response and sample_response is not None:
        preview = sample_response.strip().replace("\n", " ")
        if len(preview) > 400:
            preview = preview[:400] + "..."
        print("\nSample response preview:")
        print(preview)


def percentile(values: List[float], pct: int) -> float:
    if not values:
        return 0.0
    if pct <= 0:
        return values[0]
    if pct >= 100:
        return values[-1]
    k = (len(values) - 1) * (pct / 100.0)
    f = int(k)
    c = min(f + 1, len(values) - 1)
    if f == c:
        return values[f]
    d0 = values[f] * (c - k)
    d1 = values[c] * (k - f)
    return d0 + d1


def load_payload_template(path: Optional[Path], inline: Optional[str]) -> str:
    if path and inline:
        raise RuntimeError("Use either --payload-template or --payload-template-file, not both")
    if path:
        if not path.exists():
            raise RuntimeError(f"Payload template file not found: {path}")
        return path.read_text(encoding="utf-8")
    if inline:
        return inline
    return '{"image_url": "$data_uri"}'


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Validate DSQ snapshots and benchmark OCR HTTP endpoints."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    validate = subparsers.add_parser("validate", help="Validate DSQ snapshot alignment")
    validate.add_argument("--snapshot", required=True, help="Path to .dsq snapshot file")

    benchmark = subparsers.add_parser("benchmark", help="Benchmark OCR HTTP endpoint")
    benchmark.add_argument("--url", required=True, help="OCR HTTP endpoint URL")
    benchmark.add_argument("--image", required=True, help="Path to local image file")
    benchmark.add_argument(
        "--payload-template",
        help=("Inline JSON template using $base64, $data_uri, $mime_type, $image_path, $filename"),
    )
    benchmark.add_argument(
        "--payload-template-file",
        help="Path to JSON payload template file",
    )
    benchmark.add_argument("--requests", type=int, default=5, help="Request count")
    benchmark.add_argument("--warmup", type=int, default=1, help="Warmup request count")
    benchmark.add_argument("--timeout", type=float, default=120.0, help="HTTP timeout seconds")
    benchmark.add_argument(
        "--print-response",
        action="store_true",
        help="Print a short sample response preview",
    )

    return parser


def main(argv: Iterable[str]) -> int:
    parser = build_parser()
    args = parser.parse_args(list(argv))

    try:
        if args.command == "validate":
            snapshot_path = Path(args.snapshot).expanduser().resolve()
            validate_dsq(snapshot_path)
        elif args.command == "benchmark":
            image_path = Path(args.image).expanduser().resolve()
            if not image_path.exists():
                raise RuntimeError(f"Image file not found: {image_path}")
            if not image_path.is_file():
                raise RuntimeError(f"Image path is not a file: {image_path}")

            template = load_payload_template(
                Path(args.payload_template_file).expanduser().resolve()
                if args.payload_template_file
                else None,
                args.payload_template,
            )

            run_benchmark(
                url=args.url,
                image_path=image_path,
                payload_template=template,
                requests=args.requests,
                warmup=args.warmup,
                timeout=args.timeout,
                print_response=args.print_response,
            )
        else:
            raise RuntimeError(f"Unknown command: {args.command}")
    except DsqError as exc:
        print(f"ERROR: {exc}")
        return 2
    except Exception as exc:
        print(f"ERROR: {exc}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
