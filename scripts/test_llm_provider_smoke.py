from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

MODULE_PATH = Path(__file__).with_name("llm_provider_smoke.py")
SPEC = importlib.util.spec_from_file_location("llm_provider_smoke", MODULE_PATH)
assert SPEC is not None
assert SPEC.loader is not None
module = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = module
SPEC.loader.exec_module(module)


def test_infer_image_expected_term_groups_use_known_smoke_defaults() -> None:
    image = module.ResolvedImage(
        source="url:https://upload.wikimedia.org/wikipedia/commons/3/3f/JPEG_example_flower.jpg",
        media_type="image/jpeg",
        base64_data="abcd",
        data_uri="data:image/jpeg;base64,abcd",
    )

    assert module.infer_image_expected_term_groups(image, None) == [
        ["flower", "bloom", "hibiscus"],
        ["red", "hibiscus"],
    ]


def test_validate_image_reply_semantics_rejects_missing_required_group() -> None:
    with pytest.raises(RuntimeError, match="semantic image assertion failed"):
        module.validate_image_reply_semantics(
            "A wall covered with clusters of small pink flowers.",
            [["flower", "bloom", "hibiscus"], ["red", "hibiscus"]],
        )


def test_resolve_image_input_rejects_fake_png_fixture(tmp_path: Path) -> None:
    fake_png = tmp_path / "fake.png"
    fake_png.write_text("PNG-FIXTURE\n")

    with pytest.raises(RuntimeError, match="supported PNG/JPEG/GIF/WEBP"):
        module.resolve_image_input(str(fake_png), 10)
