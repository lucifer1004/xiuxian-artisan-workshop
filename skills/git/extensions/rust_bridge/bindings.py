"""bindings.py - Rust bindings isolation layer.

The historical Python bridge for Rust git sniffers has been removed from the
retained runtime surface. This module keeps a graceful no-op fallback so the
skill extension layer can remain importable without hard-failing.
"""

import logging

logger = logging.getLogger("xiuxian.skill.git.ext.rust.bindings")


class RustBindings:
    """Manages Rust binding imports for Git extensions."""

    _instance = None
    _checked = False
    _available = False
    _error_msg = None

    @classmethod
    def get_sniffer_cls(cls):
        """Get the Rust GitSniffer class if available."""
        if not cls._checked:
            cls._try_import()
        return cls._instance

    @classmethod
    def is_available(cls) -> bool:
        """Check if Rust bindings are available."""
        if not cls._checked:
            cls._try_import()
        return cls._available

    @classmethod
    def get_error_message(cls) -> str | None:
        """Get the error message if binding failed."""
        if not cls._checked:
            cls._try_import()
        return cls._error_msg

    @classmethod
    def _try_import(cls):
        """Attempt to import retained Rust bindings if they exist."""
        cls._checked = True
        try:
            # No retained Python Rust sniffer bridge currently exists.
            raise ImportError("retained Python Rust sniffer bridge is not available")

        except ImportError as e:
            cls._error_msg = f"Rust bindings not available: {e}"
            logger.debug(f"Rust bindings not available: {e}")
            cls._instance = None
            cls._available = False

        except Exception as e:
            cls._error_msg = f"Unexpected error loading Rust bindings: {e}"
            logger.error(f"Unexpected error loading Rust bindings: {e}")
            cls._instance = None
            cls._available = False


def get_bindings() -> type:
    """Convenience function to get the sniffer class."""
    return RustBindings.get_sniffer_cls()


def is_rust_available() -> bool:
    """Convenience function to check availability."""
    return RustBindings.is_available()
