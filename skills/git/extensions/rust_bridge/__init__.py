"""rust_bridge - Git Skill Rust Extension.

High-performance Git operations using Rust bindings.
Import this package to enable Rust acceleration for the Git skill.

Usage:
    Load the extension package directly from the git skill extension surface.

    bridge = RustAccelerator("/path/to/repo")
"""

from .accelerator import RustAccelerator, create_accelerator
from .bindings import RustBindings, get_bindings, is_rust_available

__all__ = [
    "RustAccelerator",
    "RustBindings",
    "create_accelerator",
    "get_bindings",
    "is_rust_available",
]
