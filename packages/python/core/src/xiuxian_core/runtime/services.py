"""Service Locator Pattern for the remaining Python core services.

Provides thread-safe global access to the narrowed Python compatibility services.

Usage:
    from xiuxian_core.runtime.services import get_librarian

    async def my_cli_entrypoint():
        librarian = get_librarian()
        if librarian:
            results = librarian.query("...")
"""

from __future__ import annotations

from typing import Any, TypeVar

logger = __import__("logging").getLogger(__name__)

T = TypeVar("T")


class ServiceRegistry:
    """Thread-safe service registry using Service Locator pattern.

    Allows stateless skills to access core services without direct dependencies.
    """

    _instance: ServiceRegistry | None = None
    _services: dict[str, Any] = {}
    _missing_warned: set[str] = set()
    _lock: __import__("threading").Lock = __import__("threading").Lock()

    def __new__(cls) -> ServiceRegistry:
        if cls._instance is None:
            with cls._lock:
                if cls._instance is None:
                    cls._instance = super().__new__(cls)
        return cls._instance

    @classmethod
    def register(cls, name: str, service: Any) -> None:
        """Register a core service singleton.

        Args:
            name: Service identifier (e.g., "librarian", "skill_manager")
            service: Service instance
        """
        with cls._lock:
            cls._services[name] = service
            cls._missing_warned.discard(name)
        logger.debug(f"Service registered: {name}")

    @classmethod
    def get(cls, name: str) -> Any | None:
        """Retrieve a service by name.

        Args:
            name: Service identifier

        Returns:
            Service instance or None if not registered
        """
        with cls._lock:
            service = cls._services.get(name)
            should_warn = False
            if service is None and name not in cls._missing_warned:
                cls._missing_warned.add(name)
                should_warn = True

        if should_warn:
            logger.debug("Service '%s' requested but not found in registry.", name)
        return service

    @classmethod
    def unregister(cls, name: str) -> None:
        """Remove a service from the registry.

        Args:
            name: Service identifier
        """
        with cls._lock:
            cls._services.pop(name, None)
            cls._missing_warned.discard(name)
        logger.debug(f"Service unregistered: {name}")

    @classmethod
    def clear(cls) -> None:
        """Remove all registered services."""
        with cls._lock:
            cls._services.clear()
            cls._missing_warned.clear()
        logger.debug("Service registry cleared")

    @classmethod
    def list_services(cls) -> list[str]:
        """List all registered service names."""
        with cls._lock:
            return list(cls._services.keys())


# =============================================================================
# Convenience Accessors
# =============================================================================


def get_librarian() -> None:
    """Python local librarian has been removed."""
    raise RuntimeError(
        "Python librarian has been removed. Use Rust/Wendao over Arrow Flight "
        "for knowledge ingestion and retrieval."
    )


def get_embedding_service() -> EmbeddingService | None:
    """Get the global EmbeddingService.

    Returns:
        EmbeddingService instance or None
    """
    from xiuxian_foundation.services.embedding import EmbeddingService

    service = ServiceRegistry.get("embedding")
    if service is not None and not isinstance(service, EmbeddingService):
        logger.warning("Service 'embedding' is not an EmbeddingService instance")
        return None
    return service


# =============================================================================
# Lazy Initialization Helpers
# =============================================================================


def ensure_librarian(project_root: str = ".") -> None:
    """Python local librarian has been removed."""
    _ = project_root
    raise RuntimeError(
        "Python librarian has been removed. Use Rust/Wendao over Arrow Flight "
        "for knowledge ingestion and retrieval."
    )


# Re-exports
__all__ = [
    "ServiceRegistry",
    "ensure_librarian",
    "get_embedding_service",
    "get_librarian",
]
