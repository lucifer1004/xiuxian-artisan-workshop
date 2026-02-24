#!/usr/bin/env python3
"""Read telegram.* settings via unified settings loader."""

from __future__ import annotations

import argparse

from omni.foundation.config.settings import get_setting


def read_telegram_setting(key: str) -> str:
    value = get_setting(f"telegram.{key}")
    if value is None:
        return ""
    text = str(value).strip()
    return text


def main() -> int:
    parser = argparse.ArgumentParser(description="Read telegram setting value")
    parser.add_argument("--key", required=True, help="telegram setting key (without prefix)")
    args = parser.parse_args()
    print(read_telegram_setting(str(args.key)), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
