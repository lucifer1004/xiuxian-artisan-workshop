"""
link_graph_enhancer.py - Secondary enhancement layer for LinkGraph query results.

Takes raw LinkGraph query results and enriches them using Python-side analysis:
1. Extract entity references from wikilinks ([[Entity#type]])
2. Parse YAML frontmatter for structured metadata
3. Infer lightweight relationships from note structure
4. Return enriched results with entity context and relationship data

Architecture:
    LinkGraph backend (primary engine) → raw notes
        ↓
    LinkGraphEnhancer (this module) → Python enhancement layer
        ↓
    Enriched results with entities, relations, and frontmatter metadata

Usage:
    from xiuxian_rag.link_graph_enhancer import LinkGraphEnhancer

    enhancer = LinkGraphEnhancer()
    enriched = enhancer.enhance_notes(notes)
"""

from __future__ import annotations

import logging
import re
from dataclasses import dataclass, field
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from pathlib import Path

    from .link_graph.models import LinkGraphNote

logger = logging.getLogger(__name__)


# ---------------------------------------------------------------------------
# Data classes
# ---------------------------------------------------------------------------


@dataclass
class FrontmatterData:
    """Parsed YAML frontmatter from a markdown note."""

    title: str | None = None
    description: str | None = None
    name: str | None = None
    category: str | None = None
    tags: list[str] = field(default_factory=list)
    routing_keywords: list[str] = field(default_factory=list)
    intents: list[str] = field(default_factory=list)
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class EntityRef:
    """An entity reference extracted from note content."""

    name: str
    entity_type: str | None = None
    original: str = ""


@dataclass
class EnrichedNote:
    """A LinkGraph note enriched with secondary analysis."""

    note: LinkGraphNote
    frontmatter: FrontmatterData
    entity_refs: list[EntityRef]
    ref_stats: dict[str, Any]
    # Relationships inferred from this note
    relations: list[dict[str, str]]


# ---------------------------------------------------------------------------
# Frontmatter parser (Python fallback for xiuxian-skills)
# ---------------------------------------------------------------------------

_FM_RE = re.compile(r"\A---\s*\n(.*?)\n---\s*\n", re.DOTALL)


def _parse_frontmatter(content: str) -> FrontmatterData:
    """Extract and parse YAML frontmatter from markdown content."""
    if not content:
        return FrontmatterData()

    m = _FM_RE.match(content)
    if not m:
        return FrontmatterData()

    yaml_text = m.group(1)
    try:
        import yaml

        data = yaml.safe_load(yaml_text) or {}
    except Exception:
        return FrontmatterData()

    if not isinstance(data, dict):
        return FrontmatterData()

    metadata = data.get("metadata", {}) or {}

    return FrontmatterData(
        title=data.get("title"),
        description=data.get("description"),
        name=data.get("name"),
        category=data.get("category"),
        tags=data.get("tags") or metadata.get("tags") or [],
        routing_keywords=metadata.get("routing_keywords", []),
        intents=metadata.get("intents", []),
        raw=data,
    )


# ---------------------------------------------------------------------------
# Python fallback for entity extraction
# ---------------------------------------------------------------------------

_WIKILINK_RE = re.compile(r"\[\[([^\]#|]+)(?:#([^\]#|]+))?(?:\|[^\]]+)?\]\]")


def _extract_entity_refs_py(content: str) -> list[EntityRef]:
    """Pure-Python fallback for extracting entity references from wikilinks."""
    seen: set[str] = set()
    refs: list[EntityRef] = []
    for m in _WIKILINK_RE.finditer(content):
        name = m.group(1).strip()
        etype = m.group(2).strip() if m.group(2) else None
        if name not in seen:
            seen.add(name)
            refs.append(EntityRef(name=name, entity_type=etype, original=m.group(0)))
    return refs


# ---------------------------------------------------------------------------
# LinkGraphEnhancer
# ---------------------------------------------------------------------------


class LinkGraphEnhancer:
    """Secondary enhancement layer for LinkGraph query results.

    Responsibilities (things the base LinkGraph search cannot do alone):
    - Extract typed entity references from [[wikilinks]]
    - Parse YAML frontmatter into structured metadata
    - Infer lightweight note/entity relationships
    - Compute reference statistics for ranking/scoring
    """

    def __init__(self) -> None:
        """Initialize a Python-only enhancer."""

    # ------------------------------------------------------------------
    # Core: enhance a batch of LinkGraph notes
    # ------------------------------------------------------------------

    def enhance_notes(self, notes: list[LinkGraphNote]) -> list[EnrichedNote]:
        """Enhance a batch of LinkGraph notes with secondary analysis.

        Args:
            notes: Raw LinkGraph notes from backend queries.

        Returns:
            List of EnrichedNote with frontmatter, entities, and relations.
        """
        return [self._enhance_note_python(note) for note in notes]

    def enhance_note(self, note: LinkGraphNote) -> EnrichedNote:
        """Enhance a single LinkGraph note.

        Args:
            note: Raw LinkGraph note.

        Returns:
            EnrichedNote with full secondary analysis.
        """
        return self._enhance_note_python(note)

    # ------------------------------------------------------------------
    # Python fallback path
    # ------------------------------------------------------------------

    def _enhance_note_python(self, note: LinkGraphNote) -> EnrichedNote:
        """Enhance using pure-Python implementation (fallback)."""
        content = note.raw_content or ""

        fm = _parse_frontmatter(content)
        entity_refs = self._extract_entities(content)
        ref_stats = self._get_ref_stats(content)
        relations = self._infer_relations(note, fm, entity_refs)

        return EnrichedNote(
            note=note,
            frontmatter=fm,
            entity_refs=entity_refs,
            ref_stats=ref_stats,
            relations=relations,
        )

    # ------------------------------------------------------------------
    # Entity extraction
    # ------------------------------------------------------------------

    def _extract_entities(self, content: str) -> list[EntityRef]:
        """Extract entity references from markdown content."""
        return _extract_entity_refs_py(content)

    def _get_ref_stats(self, content: str) -> dict[str, Any]:
        """Get reference statistics from markdown content."""
        refs = _extract_entity_refs_py(content)
        type_counts: dict[str, int] = {}
        for r in refs:
            t = r.entity_type or "none"
            type_counts[t] = type_counts.get(t, 0) + 1
        return {
            "total_refs": len(refs),
            "unique_entities": len(refs),
            "by_type": list(type_counts.items()),
        }

    # ------------------------------------------------------------------
    # Relation inference
    # ------------------------------------------------------------------

    def _infer_relations(
        self,
        note: LinkGraphNote,
        fm: FrontmatterData,
        entity_refs: list[EntityRef],
    ) -> list[dict[str, str]]:
        """Infer relations from note structure.

        Relations inferred:
        - DOCUMENTED_IN: Entity refs → this document
        - CONTAINS: Skill SKILL.md → its tools (from frontmatter)
        - RELATED_TO: Notes sharing tags
        - USES: From routing_keywords and intents
        """
        relations: list[dict[str, str]] = []
        doc_name = note.title or note.filename_stem or note.path

        # Entity refs → DOCUMENTED_IN
        for ref in entity_refs:
            relations.append(
                {
                    "source": ref.name,
                    "target": doc_name,
                    "relation_type": "DOCUMENTED_IN",
                    "description": f"{ref.name} documented in {doc_name}",
                }
            )

        # Skill frontmatter → CONTAINS
        if fm.name and "SKILL" in (note.filename_stem or "").upper():
            relations.append(
                {
                    "source": fm.name,
                    "target": doc_name,
                    "relation_type": "CONTAINS",
                    "description": f"Skill {fm.name} defined in {doc_name}",
                }
            )

        # Tags → potential RELATED_TO (stored for later graph use)
        for tag in fm.tags:
            relations.append(
                {
                    "source": doc_name,
                    "target": f"tag:{tag}",
                    "relation_type": "RELATED_TO",
                    "description": f"{doc_name} tagged with {tag}",
                }
            )

        return relations


# ---------------------------------------------------------------------------
# Factory
# ---------------------------------------------------------------------------

def get_link_graph_enhancer() -> LinkGraphEnhancer:
    """Create a LinkGraphEnhancer instance.

    Returns:
        LinkGraphEnhancer instance.
    """
    return LinkGraphEnhancer()


__all__ = [
    "EnrichedNote",
    "EntityRef",
    "FrontmatterData",
    "LinkGraphEnhancer",
    "get_link_graph_enhancer",
]
