"""Concrete context providers for the narrowed Python context layer."""

from __future__ import annotations

from pathlib import Path
from typing import Any, ClassVar

from xiuxian_foundation.config.logging import get_logger

from .base import ContextProvider, ContextResult

logger = get_logger("xiuxian_core.context.providers")


class SystemPersonaProvider(ContextProvider):
    """Minimal persona provider retained for Rust-authoritative context assembly."""

    DEFAULT_PERSONAS: ClassVar[dict[str, str]] = {
        "architect": "<role>You are a master software architect.</role>",
        "developer": "<role>You are an expert developer.</role>",
        "researcher": "<role>You are a thorough researcher.</role>",
    }

    def __init__(self, role: str = "architect") -> None:
        self.role = role
        self._content: str | None = None
        self._knowledge_content: str | None = None

    async def provide(self, state: dict[str, Any], budget: int) -> ContextResult | None:
        if self._content is None:
            self._content = self.DEFAULT_PERSONAS.get(
                self.role,
                f"<role>You are {self.role}.</role>",
            )

        if self._knowledge_content is None:
            try:
                from xiuxian_foundation.config import get_config_paths, get_setting

                prompt_path = get_setting("prompts.system_core") or get_setting(
                    "prompts.core_path",
                    "assets/prompts/system_core.md",
                )
                raw = Path(str(prompt_path))
                prompt_file = raw if raw.is_absolute() else get_config_paths().project_root / raw
            except (ImportError, Exception):
                from xiuxian_foundation.runtime.gitops import get_project_root

                prompt_file = get_project_root() / "assets/prompts/system_core.md"

            self._knowledge_content = (
                prompt_file.read_text() if prompt_file.exists() else ""
            )

        content = (
            f"{self._content}\n\n<knowledge_system>\n{self._knowledge_content}\n</knowledge_system>"
        )
        return ContextResult(
            content=content,
            token_count=len(content.split()),
            name="persona",
            priority=0,
        )


__all__ = ["SystemPersonaProvider"]
