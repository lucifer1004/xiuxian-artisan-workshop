"""
skill/scripts/discovery.py - Tool Discovery Commands

Unified discovery system for Agent capabilities.
Replaces the old suggest/discover dual-tool pattern with a single powerful discover.

Key Features:
- Hybrid search (Rust-native via HybridSearch)
- Usage templates to prevent parameter errors
- "Anti-hallucination" quick guide for LLM
"""

import re
from typing import Any

from omni.foundation.api.decorators import skill_command


@skill_command(
    name="discover",
    category="system",
    description="""
    [CRITICAL] Capability Discovery & Intent Resolver - The Agent's PRIMARY Entry Point.

    MANDATORY WORKFLOW: This tool is the EXCLUSIVE gateway for solving any task. It maps high-level natural language goals to specific, executable @omni commands.

    CORE RESPONSIBILITIES:
    1. INTENT MAPPING: Converts vague requests (e.g., "debug network", "optimize rust") into concrete tool sequences.
    2. GLOBAL REGISTRY ACCESS: Searches the entire Skill Registry (Active + Inactive). If a tool is found but not loaded, it provides `jit_install` instructions.
    3. SYNTAX ENFORCEMENT: Resolves the EXACT @omni(...) invocation template. Direct @omni calls are FORBIDDEN without first retrieving the template from discovery.
    4. ARCHITECTURAL ORIENTATION: Use this at the START of every session or new sub-task to identify available "superpowers" before planning.

    WHEN TO USE:
    - To find out *how* to perform a task (e.g., "how to analyze a pcap").
    - To check if a specific capability (e.g., "image processing") exists.
    - To get the correct parameter schema for a tool.
    - Whenever you encounter a new domain you haven't worked with in the current session.

    Args:
        - intent: str - The natural language goal or action (required).
        - limit: int = 5 - Max results to return (increase for complex/ambiguous tasks).

    Returns:
        A structured map containing:
        - 'quick_guide': Direct usage templates to copy and paste.
        - 'details': Full metadata, descriptions, and scores for each tool.
    """,
)
async def discover(intent: str, limit: int = 3) -> dict[str, Any]:
    """
    Unified discovery tool using the Grand Unified Router (OmniRouter).

    This is the "Google for Agent Tools" - always consult when unsure.
    """
    from omni.core.router.main import RouterRegistry

    router = RouterRegistry.get()

    # Use the Grand Unified Router's Hybrid logic
    results = await router.route_hybrid(query=intent, limit=limit, threshold=0.1, keyword_only=True)

    if not results:
        return {
            "status": "not_found",
            "message": f"No specific tools match '{intent}'.",
            "suggestions": [
                "Try a broader query (e.g., 'file' instead of 'json parser')",
                "Use `skill.list_index` to see all available skills",
                "Fallback to `terminal.run_command` if you need to explore manually",
            ],
        }

    # Construct "Intelligent Discovery Report"
    import hashlib
    import json

    def _schema_digest(schema: dict[str, Any]) -> str:
        if not isinstance(schema, dict) or not schema:
            return "sha256:empty"
        canonical = json.dumps(schema, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
        return f"sha256:{hashlib.sha256(canonical.encode('utf-8')).hexdigest()}"

    def _map_type(type_hint: str) -> str:
        lower = type_hint.lower()
        if "bool" in lower:
            return "boolean"
        if "int" in lower:
            return "integer"
        if "float" in lower or "double" in lower:
            return "number"
        if "list" in lower or "array" in lower:
            return "array"
        if "dict" in lower or "object" in lower:
            return "object"
        return "string"

    def _infer_schema_from_description(text: str) -> dict[str, Any]:
        if not text:
            return {}
        props: dict[str, dict[str, str]] = {}
        required: list[str] = []
        in_args = False
        for line in text.splitlines():
            stripped = line.strip()
            lower = stripped.lower()
            if lower.startswith("args:"):
                in_args = True
                continue
            if not in_args:
                continue
            if lower.startswith(("returns:", "example:", "examples:")):
                break
            match = re.match(r"^[\-\*]\s*`?([a-zA-Z_][\w]*)`?\s*:\s*([a-zA-Z_\[\]\|]+)", stripped)
            if not match:
                continue
            name = match.group(1)
            hint = match.group(2)
            props[name] = {"type": _map_type(hint)}
            is_required = "(required)" in lower
            is_optional = "optional" in lower or "default" in lower or "=" in stripped
            if is_required and not is_optional:
                required.append(name)

        if not props:
            return {}
        return {
            "type": "object",
            "properties": props,
            "required": required,
        }

    details = []

    for r in results:
        full_id = f"{r.skill_name}.{r.command_name}"
        description = str(getattr(r, "description", "") or "")
        file_path = str(getattr(r, "file_path", "") or "")
        raw_schema = getattr(r, "input_schema", {})
        input_schema = dict(raw_schema) if isinstance(raw_schema, dict) else {}
        if not input_schema and description:
            input_schema = _infer_schema_from_description(description)

        # 1. Calculate direct path to SKILL.md
        from omni.foundation.config.skills import SKILLS_DIR

        doc_path = ""
        try:
            potential_doc = SKILLS_DIR.definition_file(r.skill_name)
            if potential_doc.exists():
                doc_path = str(potential_doc)
        except Exception:
            pass

        # 2. Generate EXACT usage template (SSOT)
        try:
            props = input_schema.get("properties", {})
            required = input_schema.get("required", [])
            args = {}
            for p_name, p_meta in props.items():
                if p_name in required:
                    p_type = p_meta.get("type", "string")
                    args[p_name] = f"<{p_name}: {p_type}>"
                else:
                    if len(args) < 5:
                        args[p_name] = f"<{p_name}?>"

            usage = f'@omni("{full_id}", {json.dumps(args)})'
        except Exception:
            usage = f'@omni("{full_id}", {{...}})'

        # 3. Documentation hints
        doc_cues = []
        if doc_path:
            doc_cues.append(f"Full manual available at: {doc_path}")

        confidence_raw = getattr(r, "confidence", "low")
        if hasattr(confidence_raw, "value"):
            confidence = str(confidence_raw.value)
        else:
            confidence = str(confidence_raw)
        score = float(getattr(r, "score", 0.0))
        final_score = float(getattr(r, "final_score", score) or score)

        ranking_reason = str(
            getattr(r, "ranking_reason", "")
            or f"confidence={confidence} | score={score:.3f} | final_score={final_score:.3f}"
        )
        input_schema_digest = str(
            getattr(r, "input_schema_digest", "") or _schema_digest(input_schema)
        )

        details.append(
            {
                "tool": full_id,
                "score": score,
                "final_score": final_score,
                "confidence": confidence,
                "ranking_reason": ranking_reason,
                "input_schema_digest": input_schema_digest,
                "description": description,
                "usage": usage,
                "documentation_path": doc_path,
                "source_code_path": file_path,
                "documentation_hints": doc_cues,
                "advice": "Check the usage pattern. If arguments are complex, read 'documentation_path'.",
            }
        )

    return {
        "status": "success",
        "intent_matched": intent,
        "discovered_capabilities": details,
        "protocol_reminder": "NEVER guess parameters. Use the EXACT usage strings provided above.",
    }


@skill_command(
    name="jit_install",
    category="workflow",
    description="""
    Install and load a skill from the skill index on-demand.

    Args:
        - skill_id: str - The unique identifier of the skill to install (required)
        - auto_load: bool = true - If true, automatically load after installation

    Returns:
        Status message confirming the installation request.
    """,
)
def jit_install(skill_id: str, auto_load: bool = True) -> str:
    return f"Installing skill: {skill_id} (auto_load={auto_load})"


@skill_command(
    name="list_index",
    category="view",
    description="""
    List all skills in the known skills index (installed and available).

    Args:
        - None

    Returns:
        Formatted list with total skill count and collection info.
    """,
)
async def list_index() -> str:
    from omni.core.skills.discovery import SkillDiscoveryService

    service = SkillDiscoveryService()
    skills = await service.discover_all()

    lines = ["Skills Index:", ""]
    lines.append(f"Total skills: {len(skills)}")

    for skill in skills:
        lines.append(f"- {skill.name}")

    return "\n".join(lines)
