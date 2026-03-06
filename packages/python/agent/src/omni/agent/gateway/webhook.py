"""Deprecated Python gateway webhook entrypoint.

The runtime loop has been consolidated into Rust (`xiuxian-daochang gateway`).
Python webhook app creation is intentionally decommissioned to prevent
dual-loop drift between Python and Rust runtimes.
"""

from __future__ import annotations

from typing import Any


def create_webhook_app(kernel: Any, enable_cors: bool = True):
    """Raise a clear error: Python webhook app is decommissioned."""
    raise RuntimeError(
        "Python webhook runtime is decommissioned. "
        "Use Rust gateway: `xiuxian-daochang gateway --bind <host:port>`."
    )


__all__ = ["create_webhook_app"]
