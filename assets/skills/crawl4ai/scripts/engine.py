#!/usr/bin/env python3
"""
engine.py - Crawl4ai execution engine

This script runs in an isolated uv environment, allowing heavy dependencies
like crawl4ai without polluting the main agent runtime.

Output Format:
    JSON to stdout: {"success": true, "content": "...", "metadata": {...}}

Architecture:
    - Heavy imports are lazy-loaded inside functions
    - Called via run_skill_command from crawl_url.py

Features:
    - Skeleton extraction for large documents (Skeleton Planning Pattern)
    - Token-aware chunking
    - Lazy loading of large content

Usage:
    # Via run_skill_command (automatic uv run):
    from omni.foundation.runtime.isolation import run_skill_command
    result = run_skill_command(skill_dir, "engine.py", {"url": "..."})

    # Direct CLI (for testing):
    cd assets/skills/crawl4ai && VIRTUAL_ENV=.venv UV_PROJECT_ENVIRONMENT=.venv uv run python scripts/engine.py --url https://example.com
"""

import asyncio
import io
import json
import re
import sys
from contextlib import suppress
from html import unescape
from pathlib import Path
from typing import Any
from urllib.parse import unquote, urlparse

# ============================================================================
# SKELETON EXTRACTION - Fast Markdown Structure Analysis
# ============================================================================

_WORKER_LOOP: asyncio.AbstractEventLoop | None = None
_WORKER_HTTP_CRAWLER: Any | None = None


def extract_skeleton(markdown_text: str, content_handle: str | None = None) -> dict:
    """
    Extract document skeleton (TOC) without loading full content.

    This is the core of the Skeleton Planning Pattern - it provides LLM with
    a lightweight view of document structure (~500 tokens) instead of
    dumping entire content (~100k tokens).

    Args:
        markdown_text: The markdown content to analyze
        content_handle: Optional file path or ID for lazy loading content later

    Returns:
        dict with:
        - skeleton: List of section headers with metadata
        - stats: Document statistics
        - content_handle: Reference for lazy loading chunks
    """
    lines = markdown_text.split("\n")
    skeleton = []
    for i, line in enumerate(lines):
        # Detect headers (# ## ### etc)
        header_match = re.match(r"^(#{1,6})\s+(.+)$", line)
        if header_match:
            level = len(header_match.group(1))
            title = header_match.group(2).strip()

            # Calculate approximate position in document
            position = i / len(lines) if lines else 0

            skeleton.append(
                {
                    "index": len(skeleton),
                    "level": level,
                    "title": title,
                    "line_start": i,
                    "position": position,
                }
            )

    # Estimate tokens per section (rough approximation: 4 chars per token)
    for section in skeleton:
        if section["index"] < len(skeleton) - 1:
            next_start = skeleton[section["index"] + 1]["line_start"]
            section["line_end"] = next_start - 1
        else:
            section["line_end"] = len(lines) - 1

        # Count actual characters in this section
        section_lines = lines[section["line_start"] : section["line_end"] + 1]
        char_count = sum(len(line) for line in section_lines)
        section["approx_chars"] = char_count
        section["approx_tokens"] = max(1, char_count // 4)  # At least 1 token

    # Document statistics
    total_chars = len(markdown_text)
    stats = {
        "total_chars": total_chars,
        "total_tokens_approx": total_chars // 4,
        "total_lines": len(lines),
        "header_count": len(skeleton),
        "max_depth": max((s["level"] for s in skeleton), default=0),
        "content_handle": content_handle,
    }

    return {
        "skeleton": skeleton,
        "stats": stats,
    }


def extract_chunk(markdown_text: str, line_start: int, line_end: int | None = None) -> str:
    """
    Extract a specific chunk of the markdown by line numbers.

    This enables lazy loading - we only extract the sections LLM
    decided to process, not the entire document.

    Args:
        markdown_text: Full markdown content
        line_start: Starting line index
        line_end: Ending line index (inclusive, optional)

    Returns:
        str: The extracted chunk content
    """
    lines = markdown_text.split("\n")
    if line_end is None:
        line_end = len(lines) - 1

    # Ensure bounds
    line_start = max(0, line_start)
    line_end = min(len(lines) - 1, line_end)

    return "\n".join(lines[line_start : line_end + 1])


# ============================================================================
# MAIN CRAWL IMPLEMENTATION
# ============================================================================


def _extract_title_from_html(raw_html: str) -> str | None:
    """Extract <title> from raw HTML."""
    title_match = re.search(
        r"<title[^>]*>(.*?)</title>",
        raw_html,
        flags=re.IGNORECASE | re.DOTALL,
    )
    if not title_match:
        return None
    title = unescape(title_match.group(1)).strip()
    return title or None


def _html_to_markdown_minimal(raw_html: str) -> str:
    """Convert simple HTML into lightweight markdown for local fixture fast-path."""
    normalized = raw_html.replace("\r\n", "\n")

    for level in range(6, 0, -1):
        pattern = re.compile(
            rf"<h{level}[^>]*>(.*?)</h{level}>",
            flags=re.IGNORECASE | re.DOTALL,
        )
        normalized = pattern.sub(
            lambda m,
            lvl=level: f"\n{'#' * lvl} {unescape(_strip_html_tags(m.group(1))).strip()}\n",
            normalized,
        )

    normalized = re.sub(
        r"<li[^>]*>(.*?)</li>",
        lambda m: f"\n- {unescape(_strip_html_tags(m.group(1))).strip()}",
        normalized,
        flags=re.IGNORECASE | re.DOTALL,
    )
    normalized = re.sub(
        r"<p[^>]*>(.*?)</p>",
        lambda m: f"\n{unescape(_strip_html_tags(m.group(1))).strip()}\n",
        normalized,
        flags=re.IGNORECASE | re.DOTALL,
    )
    normalized = re.sub(r"<br\\s*/?>", "\n", normalized, flags=re.IGNORECASE)
    normalized = _strip_html_tags(normalized)

    cleaned_lines = [line.rstrip() for line in normalized.split("\n")]
    cleaned = "\n".join(line for line in cleaned_lines if line.strip())
    return (cleaned.strip() + "\n") if cleaned.strip() else ""


def _strip_html_tags(text: str) -> str:
    """Strip HTML tags from a string."""
    return re.sub(r"<[^>]+>", "", text, flags=re.DOTALL)


def _try_local_file_fast_path(url: str, *, fit_markdown: bool) -> dict[str, Any] | None:
    """Serve file:// URLs without crawl4ai runtime for deterministic local benchmarking."""
    parsed = urlparse(url)
    if parsed.scheme.lower() != "file":
        return None
    local_path = Path(unquote(parsed.path))
    if not local_path.exists() or not local_path.is_file():
        return {
            "success": False,
            "url": url,
            "content": "",
            "error": f"Local file not found: {local_path}",
            "metadata": None,
            "crawled_urls": None,
        }

    try:
        raw_text = local_path.read_text(encoding="utf-8")
    except UnicodeDecodeError:
        raw_text = local_path.read_text(encoding="utf-8", errors="replace")

    suffix = local_path.suffix.lower()
    is_html = suffix in {".html", ".htm"}
    content = _html_to_markdown_minimal(raw_text) if (fit_markdown and is_html) else raw_text

    title = _extract_title_from_html(raw_text) if is_html else local_path.name
    return {
        "success": True,
        "url": url,
        "content": content,
        "error": "",
        "metadata": {
            "title": title,
            "description": None,
        },
        "crawled_urls": None,
    }


async def _get_worker_http_crawler() -> Any:
    """Get a started HTTP crawler instance reused by persistent worker requests."""
    global _WORKER_HTTP_CRAWLER
    if _WORKER_HTTP_CRAWLER is not None:
        return _WORKER_HTTP_CRAWLER

    from crawl4ai import AsyncWebCrawler, HTTPCrawlerConfig
    from crawl4ai.async_crawler_strategy import AsyncHTTPCrawlerStrategy

    http_strategy = AsyncHTTPCrawlerStrategy(
        browser_config=HTTPCrawlerConfig(
            method="GET",
            follow_redirects=True,
            verify_ssl=False,
        )
    )
    crawler = AsyncWebCrawler(crawler_strategy=http_strategy, verbose=False)
    await crawler.start()
    _WORKER_HTTP_CRAWLER = crawler
    return crawler


async def _close_worker_runtime() -> None:
    """Close reusable async resources held by persistent worker."""
    global _WORKER_HTTP_CRAWLER
    crawler = _WORKER_HTTP_CRAWLER
    _WORKER_HTTP_CRAWLER = None
    if crawler is not None:
        await crawler.close()


def _run_async(coro: Any) -> Any:
    """Run async coroutine with reusable loop in worker mode."""
    if _WORKER_LOOP is None:
        return asyncio.run(coro)
    return _WORKER_LOOP.run_until_complete(coro)


async def _crawl_url_impl(
    url: str,
    fit_markdown: bool = True,
    max_depth: int = 0,
) -> dict:
    """
    Internal implementation - runs in isolated uv environment.

    Args:
        url: Target URL to crawl
        fit_markdown: Whether to clean and simplify the markdown (default: True)
        max_depth: Maximum crawling depth (0 = single page only, >0 = crawl linked pages)

    Returns:
        dict with keys:
        - success: bool
        - url: str
        - content: str (markdown)
        - error: str (if success is False)
        - metadata: dict (title, description)
        - crawled_urls: list[str] (urls crawled when max_depth > 0)
    """
    # Capture stdout during crawl to prevent progress bars from polluting JSON output
    old_stdout = sys.stdout
    sys.stdout = io.StringIO()

    try:
        if max_depth <= 0:
            local_fast = _try_local_file_fast_path(url, fit_markdown=fit_markdown)
            if local_fast is not None:
                sys.stdout = old_stdout
                return local_fast

        from crawl4ai import AsyncWebCrawler, CrawlerRunConfig

        # Shared low-latency defaults for both HTTP and browser strategies.
        base_config = CrawlerRunConfig(
            wait_until="domcontentloaded",
            delay_before_return_html=0.0,
            mean_delay=0.0,
            max_range=0.0,
            verbose=False,
            log_console=False,
        )

        if max_depth > 0:
            from crawl4ai.deep_crawling import BFSDeepCrawlStrategy

            async with AsyncWebCrawler(verbose=False) as crawler:
                deep_config = CrawlerRunConfig(
                    wait_until="domcontentloaded",
                    delay_before_return_html=0.0,
                    mean_delay=0.0,
                    max_range=0.0,
                    verbose=False,
                    log_console=False,
                    deep_crawl_strategy=BFSDeepCrawlStrategy(
                        max_depth=max_depth,
                        include_external=False,
                        max_pages=20,
                    ),
                )
                result = await crawler.arun(url=url, config=deep_config)
            # Deep crawl may return a generator - convert to list
            results = (
                list(result)
                if hasattr(result, "__iter__") and not isinstance(result, dict)
                else [result]
            )
            # Combine markdown from all pages
            all_content: list[str] = []
            all_urls: list[str] = []
            for r in results:
                if r.success:
                    all_content.append(_extract_result_markdown(r, fit_markdown=fit_markdown))
                    all_urls.append(str(r.url))
            combined_content = "\n\n---\n\n".join(all_content)
            first_result = results[0] if results else None
            success = any(r.success for r in results)
            error_msg = first_result.error_message if first_result else ""
            metadata = first_result.metadata if first_result else None
            sys.stdout = old_stdout  # Restore stdout before return
            return {
                "success": success,
                "url": url,
                "content": combined_content,
                "error": error_msg,
                "metadata": {
                    "title": metadata.get("title") if metadata else None,
                    "description": metadata.get("description") if metadata else None,
                },
                "crawled_urls": all_urls if all_urls else None,
            }

        # Single-page crawl fast path:
        # Use HTTP strategy to avoid browser cold-start on every isolated invocation.
        if _WORKER_LOOP is not None:
            crawler = await _get_worker_http_crawler()
            result = await crawler.arun(url=url, config=base_config)
        else:
            from crawl4ai import HTTPCrawlerConfig
            from crawl4ai.async_crawler_strategy import AsyncHTTPCrawlerStrategy

            http_strategy = AsyncHTTPCrawlerStrategy(
                browser_config=HTTPCrawlerConfig(
                    method="GET",
                    follow_redirects=True,
                    verify_ssl=False,
                )
            )
            async with AsyncWebCrawler(crawler_strategy=http_strategy, verbose=False) as crawler:
                result = await crawler.arun(url=url, config=base_config)

        # Discard captured stdout (progress bars)
        sys.stdout = old_stdout

        # Collect crawled URLs if available
        crawled_urls = []
        if hasattr(result, "crawled_urls") and result.crawled_urls:
            crawled_urls = result.crawled_urls
        elif hasattr(result, "downloaded_urls") and result.downloaded_urls:
            crawled_urls = result.downloaded_urls

        return {
            "success": result.success,
            "url": result.url,
            "content": _extract_result_markdown(result, fit_markdown=fit_markdown),
            "error": result.error_message or "",
            "metadata": {
                "title": result.metadata.get("title") if result.metadata else None,
                "description": result.metadata.get("description") if result.metadata else None,
            },
            "crawled_urls": crawled_urls if crawled_urls else None,
        }

    except Exception as e:
        sys.stdout = old_stdout
        return {
            "success": False,
            "url": url,
            "content": "",
            "error": str(e),
            "metadata": None,
            "crawled_urls": None,
        }


def _extract_result_markdown(result: Any, *, fit_markdown: bool) -> str:
    """Normalize markdown extraction across different crawl4ai strategies."""
    markdown = getattr(result, "markdown", "")
    if fit_markdown:
        return str(markdown or "")

    raw_markdown = getattr(result, "raw_markdown", None)
    if raw_markdown:
        return str(raw_markdown)
    return str(markdown or "")


# ============================================================================
# ACTION HANDLERS
# ============================================================================


def _build_skeleton_response(result: dict[str, Any]) -> dict[str, Any]:
    """Build skeleton extraction payload."""
    if not result.get("success"):
        return result

    skeleton_result = extract_skeleton(str(result.get("content") or ""))
    skeleton = skeleton_result["skeleton"]
    stats = skeleton_result["stats"]
    return {
        "success": True,
        "url": result.get("url", ""),
        "skeleton": skeleton,
        "stats": stats,
        "metadata": result.get("metadata", {}),
    }


def _build_smart_response(
    result: dict[str, Any],
    url: str,
    chunk_plan: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    """Build smart-action response payload."""
    if not result.get("success"):
        return result

    # Extract skeleton from content (already crawled)
    content = str(result.get("content") or "")
    skeleton_result = extract_skeleton(content)
    skeleton = skeleton_result["skeleton"]
    metadata = result.get("metadata", {})
    title = metadata.get("title", url)

    # Use provided chunk_plan or fallback
    plan = chunk_plan
    if not plan:
        # Fallback: each section as its own chunk
        plan = [
            {
                "chunk_id": i,
                "section_indices": [i],
                "reason": f"Section: {s.get('title', '')}",
            }
            for i, s in enumerate(skeleton)
        ]

    # Process chunks based on chunk_plan
    processed_chunks: list[dict[str, Any]] = []
    for chunk_info in plan:
        section_indices = chunk_info.get("section_indices", [])
        chunk_content_parts: list[str] = []
        for sec_idx in section_indices:
            if isinstance(sec_idx, int) and sec_idx < len(skeleton):
                section = skeleton[sec_idx]
                line_start = int(section.get("line_start", 0))
                line_end = int(section.get("line_end", line_start))
                content_chunk = extract_chunk(content, line_start, line_end)
                chunk_content_parts.append(f"## {section.get('title', '')}\n{content_chunk}")

        combined_content = "\n\n".join(chunk_content_parts)
        processed_chunks.append(
            {
                "chunk_id": chunk_info.get("chunk_id", len(processed_chunks)),
                "reason": chunk_info.get("reason", ""),
                "content": combined_content,
                "section_indices": section_indices,
            }
        )

    # Build final summary
    final_summary = f"# {title}\n\n"
    for chunk in processed_chunks:
        final_summary += f"## Chunk {chunk.get('chunk_id', 0)}: {chunk.get('reason', '')}\n\n"
        final_summary += str(chunk.get("content", "")) + "\n\n"

    # Format output
    chunk_plan_text = []
    for chunk in plan[:10]:
        indices = chunk.get("section_indices", [])
        chunk_plan_text.append(
            f"  - Chunk {chunk.get('chunk_id', '?')}: sections {indices} - {chunk.get('reason', '')}"
        )
    if len(plan) > 10:
        chunk_plan_text.append(f"  ... and {len(plan) - 10} more chunks")

    chunks_text = []
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
        "chunk_plan": plan,
        "processed_chunks": processed_chunks,
    }


def _build_chunk_response(result: dict[str, Any], chunk_indices: list[int]) -> dict[str, Any]:
    """Build extracted chunk payload."""
    if not result.get("success"):
        return result

    skeleton_result = extract_skeleton(str(result.get("content") or ""))
    skeleton = skeleton_result["skeleton"]
    chunks = []

    for idx in chunk_indices:
        if 0 <= idx < len(skeleton):
            section = skeleton[idx]
            chunk = extract_chunk(
                str(result.get("content") or ""),
                int(section.get("line_start", 0)),
                int(section.get("line_end", section.get("line_start", 0))),
            )
            chunks.append(
                {
                    "chunk_id": idx,
                    "title": section["title"],
                    "content": chunk,
                    "approx_tokens": section["approx_tokens"],
                }
            )

    return {
        "success": True,
        "url": result.get("url", ""),
        "chunks": chunks,
        "metadata": result.get("metadata", {}),
    }


def _execute_request(payload: dict[str, Any]) -> dict[str, Any]:
    """Execute one crawl request payload."""
    url = str(payload.get("url") or "")
    if not url:
        return {"success": False, "error": "Missing URL"}

    action = str(payload.get("action") or "crawl")
    fit_markdown = bool(payload.get("fit_markdown", True))
    max_depth = int(payload.get("max_depth", 0) or 0)
    return_skeleton = bool(payload.get("return_skeleton", False))
    chunk_plan = payload.get("chunk_plan")
    raw_chunk_indices = payload.get("chunk_indices") or []
    chunk_indices: list[int] = []
    for item in raw_chunk_indices:
        try:
            chunk_indices.append(int(item))
        except (TypeError, ValueError):
            continue

    try:
        result = _run_async(_crawl_url_impl(url, fit_markdown, max_depth))
    except Exception as e:
        return {"success": False, "error": str(e)}

    if action == "skeleton" or return_skeleton:
        return _build_skeleton_response(result)
    if action == "smart":
        plan = chunk_plan if isinstance(chunk_plan, list) else None
        return _build_smart_response(result, url, plan)
    if chunk_indices:
        return _build_chunk_response(result, chunk_indices)
    return result


def _run_worker() -> None:
    """Run persistent worker loop (JSON line protocol on stdin/stdout)."""
    global _WORKER_LOOP
    _WORKER_LOOP = asyncio.new_event_loop()
    try:
        for raw in sys.stdin:
            line = raw.strip()
            if not line:
                continue
            try:
                payload = json.loads(line)
                if not isinstance(payload, dict):
                    raise TypeError("payload must be an object")
                response = _execute_request(payload)
            except Exception as e:
                response = {"success": False, "error": str(e)}
            print(json.dumps(response, default=str), flush=True)
    finally:
        if _WORKER_LOOP is not None:
            with suppress(Exception):
                _WORKER_LOOP.run_until_complete(_close_worker_runtime())
            with suppress(Exception):
                _WORKER_LOOP.close()
        _WORKER_LOOP = None


def _handle_skeleton_action(result: dict) -> None:
    """Handle skeleton extraction action."""
    print(json.dumps(_build_skeleton_response(result), default=str))


def _handle_smart_action(result: dict, url: str, chunk_plan: list | None = None) -> None:
    """Handle smart crawl action - extract chunks based on pre-computed chunk_plan.

    The chunk_plan is generated by LLM in the main MCP environment.
    This function simply executes the extraction in the isolated environment.
    """
    payload = _build_smart_response(
        result,
        url,
        chunk_plan if isinstance(chunk_plan, list) else None,
    )
    if payload.get("success"):
        print(str(payload.get("content", "")))
        return
    print(json.dumps(payload, default=str))


def _handle_chunk_action(result: dict, chunk_indices: list[int]) -> None:
    """Handle chunk extraction action."""
    print(json.dumps(_build_chunk_response(result, chunk_indices), default=str))


def main():
    """CLI entry point - supports both stdin JSON and command line args."""
    import argparse

    parser = argparse.ArgumentParser(description="Crawl4AI Engine")
    parser.add_argument("--url", type=str, help="URL to crawl")
    parser.add_argument(
        "--action", type=str, default="crawl", help="Action: crawl, skeleton, smart"
    )
    parser.add_argument(
        "--fit_markdown", type=str, default="true", help="Clean markdown (true/false)"
    )
    parser.add_argument(
        "--max_depth", type=int, default=0, help="Maximum crawling depth (0=single page)"
    )
    parser.add_argument(
        "--return_skeleton",
        nargs="?",  # Optional value
        const="true",  # Value when flag is present without argument
        default="false",  # Default value
        help="Only return document skeleton (TOC), not full content",
    )
    parser.add_argument(
        "--chunk_indices",
        type=str,
        default="",
        help="Comma-separated chunk indices to extract (e.g., '0,1,3')",
    )
    parser.add_argument(
        "--chunk_plan",
        type=str,
        default="",
        help="JSON-encoded chunk plan from LLM (for smart action)",
    )
    parser.add_argument("--stdin", action="store_true", help="Read JSON from stdin")
    parser.add_argument(
        "--worker",
        action="store_true",
        help="Run as persistent worker (one JSON request per stdin line)",
    )

    args = parser.parse_args()

    if args.worker:
        _run_worker()
        return

    # Parse URL and fit_markdown
    url = ""
    action = "crawl"
    fit_markdown = True
    max_depth = 0
    return_skeleton = False
    chunk_indices = []
    chunk_plan = None

    if args.stdin and not sys.stdin.isatty():
        # Read JSON from stdin (for uv run with pipe)
        try:
            stdin_data = sys.stdin.read()
            if stdin_data.strip():
                json_args = json.loads(stdin_data)
                url = json_args.get("url", "")
                action = json_args.get("action", "crawl")
                fit_markdown = json_args.get("fit_markdown", True)
                max_depth = json_args.get("max_depth", 0)
                return_skeleton = json_args.get("return_skeleton", False)
                if json_args.get("chunk_indices"):
                    chunk_indices = json_args["chunk_indices"]
                if json_args.get("chunk_plan"):
                    chunk_plan = json_args["chunk_plan"]
        except json.JSONDecodeError:
            pass
    else:
        # Use command line args (for run_skill_command)
        url = args.url or ""
        action = args.action or "crawl"
        fit_markdown = args.fit_markdown.lower() == "true" if args.fit_markdown else True
        max_depth = args.max_depth or 0
        # return_skeleton is now a string "true"/"false" due to nargs="?"
        return_skeleton = (
            str(args.return_skeleton).lower() == "true" if args.return_skeleton else False
        )
        if args.chunk_indices:
            chunk_indices = [int(x.strip()) for x in args.chunk_indices.split(",")]
        if args.chunk_plan:
            try:
                chunk_plan = json.loads(args.chunk_plan)
            except json.JSONDecodeError:
                chunk_plan = None

    if not url:
        print(json.dumps({"success": False, "error": "Missing URL"}, default=str))
        return

    try:
        result = _run_async(_crawl_url_impl(url, fit_markdown, max_depth))

        # Handle different actions
        if action == "skeleton" or return_skeleton:
            _handle_skeleton_action(result)
        elif action == "smart":
            _handle_smart_action(result, url, chunk_plan)
        elif chunk_indices:
            _handle_chunk_action(result, chunk_indices)
        else:
            # Default: return full content
            print(json.dumps(result, default=str))

    except Exception as e:
        print(json.dumps({"success": False, "error": str(e)}, default=str))


if __name__ == "__main__":
    main()
