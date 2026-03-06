#!/usr/bin/env python3
"""Media resolution helpers for agent channel blackbox config."""

from __future__ import annotations

import base64
import mimetypes
from pathlib import Path
from typing import Any


def _normalize_image_url(raw: str) -> str:
    candidate = raw.strip()
    if not candidate:
        raise ValueError("image url is empty")
    if not (
        candidate.startswith("https://")
        or candidate.startswith("http://")
        or candidate.startswith("data:")
    ):
        raise ValueError(
            "image url must start with https://, http://, or data: so [IMAGE:...] can be parsed"
        )
    return candidate


def _file_to_data_uri(image_file: str) -> str:
    path = Path(image_file).expanduser().resolve()
    if not path.exists():
        raise ValueError(f"image file not found: {path}")
    if not path.is_file():
        raise ValueError(f"image path is not a file: {path}")

    mime_type, _encoding = mimetypes.guess_type(path.name)
    if not mime_type:
        mime_type = "image/png"
    if not mime_type.startswith("image/"):
        raise ValueError(f"image file mime is not image/*: path={path} mime={mime_type}")

    payload = base64.b64encode(path.read_bytes()).decode("ascii")
    return f"data:{mime_type};base64,{payload}"


def resolve_image_url(args: Any) -> str | None:
    """Resolve optional image input for multimodal probe injection.

    Priority:
    1) `--image-file` -> data URI
    2) `--image-url`
    """
    image_url_raw = (getattr(args, "image_url", None) or "").strip()
    image_file_raw = (getattr(args, "image_file", None) or "").strip()

    if image_url_raw and image_file_raw:
        raise ValueError("--image-url and --image-file are mutually exclusive")

    if image_file_raw:
        return _file_to_data_uri(image_file_raw)
    if image_url_raw:
        return _normalize_image_url(image_url_raw)
    return None
