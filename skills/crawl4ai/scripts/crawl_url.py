"""
crawl4ai/scripts/ - Crawl4ai Skill Interface

This module exposes command interfaces for the main agent.
Commands are executed via Foundation's Isolation Pattern.

IMPORTANT: These function signatures are invoked from CLI/runtime entrypoints.
The actual crawl implementation is in engine.py.
"""

import importlib
import json
from pathlib import Path
from typing import Any

import structlog

from xiuxian_foundation.context_delivery import validate_chunked_action
from xiuxian_foundation.services.llm.client import InferenceClient
from skills._shared.isolation import run_script_command

log = structlog.get_logger("xiuxian.skill.crawl4ai")


def _get_skill_dir() -> Path:
    """Get the skill directory for isolation."""
    return Path(__file__).parent.parent


def _resolve_engine_helpers():
    """Load pure markdown helpers from engine module without crawl dependency imports."""
    try:
        from engine import extract_chunk, extract_skeleton

        return extract_skeleton, extract_chunk
    except ImportError:
        pass

    pkg = __package__
    if pkg:
        mod = importlib.import_module(".engine", package=pkg)
        return mod.extract_skeleton, mod.extract_chunk
    raise ImportError("Could not import extract_skeleton/extract_chunk from engine module")


def _build_smart_result(
    *,
    url: str,
    crawl_result: dict[str, Any],
    chunk_plan: list[dict[str, Any]] | None,
) -> dict[str, Any]:
    """Build smart crawl summary from already-crawled content (single crawl execution)."""
    content = str(crawl_result.get("content") or "")
    metadata_obj = crawl_result.get("metadata")
    metadata = metadata_obj if isinstance(metadata_obj, dict) else {}
    title = str(metadata.get("title") or url)

    extract_skeleton, extract_chunk = _resolve_engine_helpers()
    skeleton_result = extract_skeleton(content)
    skeleton = skeleton_result.get("skeleton", [])
    stats = skeleton_result.get("stats", {})

    plan = chunk_plan or [
        {
            "chunk_id": i,
            "section_indices": [i],
            "reason": f"Section: {section.get('title', '')}",
        }
        for i, section in enumerate(skeleton)
    ]

    processed_chunks: list[dict[str, Any]] = []
    for chunk_info in plan:
        section_indices = chunk_info.get("section_indices", [])
        chunk_content_parts: list[str] = []
        for sec_idx in section_indices:
            if not isinstance(sec_idx, int) or sec_idx < 0 or sec_idx >= len(skeleton):
                continue
            section = skeleton[sec_idx]
            line_start = int(section.get("line_start", 0))
            line_end = int(section.get("line_end", line_start))
            content_chunk = extract_chunk(content, line_start, line_end)
            chunk_content_parts.append(f"## {section.get('title', '')}\n{content_chunk}")

        processed_chunks.append(
            {
                "chunk_id": chunk_info.get("chunk_id", len(processed_chunks)),
                "reason": chunk_info.get("reason", ""),
                "content": "\n\n".join(chunk_content_parts),
                "section_indices": section_indices,
            }
        )

    final_summary = f"# {title}\n\n"
    for chunk in processed_chunks:
        final_summary += f"## Chunk {chunk.get('chunk_id', 0)}: {chunk.get('reason', '')}\n\n"
        final_summary += str(chunk.get("content", "")) + "\n\n"

    chunk_plan_text: list[str] = []
    for chunk in plan[:10]:
        indices = chunk.get("section_indices", [])
        chunk_plan_text.append(
            f"  - Chunk {chunk.get('chunk_id', '?')}: sections {indices} - {chunk.get('reason', '')}"
        )
    if len(plan) > 10:
        chunk_plan_text.append(f"  ... and {len(plan) - 10} more chunks")

    chunks_text: list[str] = []
    for chunk in processed_chunks:
        content_preview = str(chunk.get("content", ""))[:200].replace("\n", " ")
        chunks_text.append(f"  - Chunk {chunk.get('chunk_id', '?')}: {content_preview}...")

    output = f"""# Crawl Result: {title}

**URL:** {url}
**Status:** Success

## Workflow Execution

### 1. Crawl + Skeleton
Extracted {len(skeleton)} sections from document

### 2. LLM Chunking Plan
{chr(10).join(chunk_plan_text) if chunk_plan_text else "  No chunks planned"}

### 3. Processed Chunks ({len(processed_chunks)} total)
{chr(10).join(chunks_text) if chunks_text else "  No chunks processed"}

---

## Final Summary

{final_summary if final_summary else "(No summary generated)"}

---

## Raw Data

**Skeleton:** {len(skeleton)} sections
**Chunk Plan:** {len(plan)} chunks
**Processed:** {len(processed_chunks)} chunks
"""

    return {
        "success": True,
        "url": url,
        "content": output,
        "metadata": metadata,
        "skeleton": skeleton,
        "stats": stats,
        "chunk_plan": plan,
        "processed_chunks": processed_chunks,
    }


# LLM prompt for chunk planning
CHUNKING_PROMPT = """You are an intelligent document chunking planner.

## Document
- Title: {title}
- Total Sections: {section_count}

## Skeleton
{skeleton}

## Task
Create a chunking plan. Return JSON only:

{{
    "chunks": [
        {{
            "chunk_id": 0,
            "section_indices": [0, 1],
            "reason": "Introduction and overview"
        }}
    ]
}}
"""


async def _generate_chunk_plan(skeleton: list, title: str) -> list | None:
    """Generate chunk plan using LLM (runs in main environment)."""
    try:
        # Format skeleton for LLM
        skeleton_lines = []
        for i, section in enumerate(skeleton[:30]):
            indent = "  " * (section.get("level", 1) - 1)
            skeleton_lines.append(f"{indent}- [{i}] {section.get('title', '')}")
        skeleton_text = "\n".join(skeleton_lines)

        # Build prompt
        prompt = CHUNKING_PROMPT.format(
            title=title or "Untitled",
            section_count=len(skeleton),
            skeleton=skeleton_text,
        )

        # Call LLM - InferenceClient.complete() expects system_prompt and user_query
        llm = InferenceClient()
        response = llm.complete(
            system_prompt="You are an intelligent document chunking planner. Return valid JSON only.",
            user_query=prompt,
            max_tokens=2000,
        )

        # Parse JSON response
        content = response.get("content", "")
        if "```json" in content:
            content = content.split("```json")[1].split("```")[0]
        elif "```" in content:
            content = content.split("```")[1].split("```")[0]

        plan_data = json.loads(content)

        # Build chunk plan
        chunk_plan = []
        for chunk in plan_data.get("chunks", []):
            chunk_plan.append(
                {
                    "chunk_id": chunk.get("chunk_id", len(chunk_plan)),
                    "section_indices": chunk.get("section_indices", []),
                    "reason": chunk.get("reason", ""),
                    "estimated_tokens": chunk.get("estimated_tokens", 0),
                }
            )

        return chunk_plan

    except Exception as e:
        log.error("LLM chunk planning failed", error=str(e))
        return None


async def CrawlUrl(
    url: str,
    action: str = "smart",
    fit_markdown: bool = True,
    max_depth: int = 0,
    return_skeleton: bool = False,
    chunk_indices: list[int] | None = None,
) -> dict[str, Any] | str:
    """
    Crawl URL with intelligent chunking.

    Args:
        - url: Target URL to crawl
        - action: "smart" (default, LLM-planned), "skeleton" (TOC only), "crawl" (full content)
        - fit_markdown: Clean markdown output
        - max_depth: Crawl depth (0 = single page)
        - return_skeleton: Include skeleton in response
        - chunk_indices: Specific sections to extract
    """
    action_name, action_error = validate_chunked_action(
        action,
        allowed_actions={"smart", "skeleton", "crawl"},
        allow_empty=False,
    )
    if action_error is not None:
        return action_error

    # Auto-upgrade to smart mode when crawling depth > 1
    # Smart mode uses LLM to plan optimal chunking for multi-page crawls
    if max_depth > 1 and action_name == "crawl":
        action_name = "smart"
        log.info("Auto-upgraded to smart mode", max_depth=max_depth)
    # For smart action, we need to:
    # 1. First crawl to get skeleton
    # 2. Generate chunk plan with LLM
    # 3. Pass chunk_plan to engine for execution
    chunk_plan = None
    if action_name == "smart":
        # First crawl to get skeleton
        crawl_result = run_script_command(
            script_root=_get_skill_dir(),
            script_name="engine.py",
            args={
                "url": url,
                "action": "crawl",
                "fit_markdown": fit_markdown,
                "max_depth": max_depth,
            },
            persistent=True,
        )

        if not crawl_result.get("success"):
            return crawl_result

        # Generate chunk plan with LLM (in main environment)
        skeleton = crawl_result.get("skeleton", [])
        title = crawl_result.get("metadata", {}).get("title", "")

        if skeleton:
            chunk_plan = await _generate_chunk_plan(skeleton, title)

    # Pass to engine (with chunk_plan if available)
    if action_name == "smart":
        return _build_smart_result(url=url, crawl_result=crawl_result, chunk_plan=chunk_plan)

    result = run_script_command(
        script_root=_get_skill_dir(),
        script_name="engine.py",
        args={
            "url": url,
            "action": action_name,
            "fit_markdown": fit_markdown,
            "max_depth": max_depth,
            "return_skeleton": return_skeleton,
            "chunk_indices": chunk_indices or [],
            "chunk_plan": chunk_plan,
        },
        persistent=True,
    )
    return result


# Legacy alias for backward compatibility
crawl_url = CrawlUrl
