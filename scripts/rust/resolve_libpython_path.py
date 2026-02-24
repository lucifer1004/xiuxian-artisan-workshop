#!/usr/bin/env python3
"""Resolve active Python shared library path for omni-core-rs tests."""

from __future__ import annotations

import os
import sysconfig


def resolve_libpython_path() -> str:
    libdir = sysconfig.get_config_var("LIBDIR")
    ldlibrary = sysconfig.get_config_var("LDLIBRARY")
    if not libdir or not ldlibrary:
        return ""
    return os.path.join(str(libdir), str(ldlibrary))


def main() -> int:
    path = resolve_libpython_path()
    if not path:
        return 1
    print(path, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
