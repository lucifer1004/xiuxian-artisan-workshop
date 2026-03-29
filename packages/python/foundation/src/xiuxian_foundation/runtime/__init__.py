# runtime
"""
Runtime Environment Module

Provides execution environment utilities:
- isolation.py: Sidecar execution for skill scripts
- gitops.py: Git operations and project root detection
- path.py: Safe sys.path manipulation utilities

Usage:
    from xiuxian_foundation.runtime.isolation import run_skill_command
    from xiuxian_foundation.runtime.gitops import get_project_root
"""

from .gitops import (
    PROJECT,
    get_agent_dir,
    get_docs_dir,
    get_instructions_dir,
    get_project_root,
    get_spec_dir,
    get_src_dir,
    is_git_repo,
    is_project_root,
)
from .isolation import run_skill_command
from .path import temporary_sys_path
from .skill_optimization import (
    BALANCED_PROFILE,
    LATENCY_PROFILE,
    THROUGHPUT_PROFILE,
    build_preview_rows,
    clamp_float,
    clamp_int,
    compute_batch_count,
    filter_ranked_chunks,
    get_chunk_window_profile,
    is_low_signal_query,
    is_markdown_index_chunk,
    normalize_chunk_window,
    normalize_min_score,
    normalize_snippet_chars,
    parse_bool,
    parse_float,
    parse_int,
    parse_optional_int,
    resolve_bool_from_setting,
    resolve_float_from_setting,
    resolve_int_from_setting,
    resolve_optional_int_from_setting,
    slice_batch,
    split_into_batches,
)

__all__ = [
    "BALANCED_PROFILE",
    "LATENCY_PROFILE",
    "PROJECT",
    "THROUGHPUT_PROFILE",
    "build_preview_rows",
    "clamp_float",
    "clamp_int",
    "compute_batch_count",
    "filter_ranked_chunks",
    "get_agent_dir",
    "get_chunk_window_profile",
    "get_docs_dir",
    "get_instructions_dir",
    "get_project_root",
    "get_spec_dir",
    "get_src_dir",
    "is_git_repo",
    "is_low_signal_query",
    "is_markdown_index_chunk",
    "is_project_root",
    "normalize_chunk_window",
    "normalize_min_score",
    "normalize_snippet_chars",
    "parse_bool",
    "parse_float",
    "parse_int",
    "parse_optional_int",
    "resolve_bool_from_setting",
    "resolve_float_from_setting",
    "resolve_int_from_setting",
    "resolve_optional_int_from_setting",
    "run_skill_command",
    "slice_batch",
    "split_into_batches",
    "temporary_sys_path",
]
