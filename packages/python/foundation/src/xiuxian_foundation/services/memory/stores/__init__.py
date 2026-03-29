"""Memory store namespace.

Python-owned memory stores were removed with the local LanceDB path. The
remaining architecture should consume Rust-owned memory and retrieval services
over Arrow Flight instead of creating Python storage backends.
"""

__all__: list[str] = []
