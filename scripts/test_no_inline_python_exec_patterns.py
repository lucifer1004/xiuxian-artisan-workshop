from __future__ import annotations

import re
from pathlib import Path

FORBIDDEN_PATTERNS = [
    re.compile(r"\bpython3?\s+-c\b"),
    re.compile(r"\buv\s+run\s+python\s+-c\b"),
    re.compile(r'"\\?\$PYTHON_BIN"\s+-c\b'),
    re.compile(r"<<'PY'"),
    re.compile(r"<<PY"),
]


def _iter_target_files(repo_root: Path) -> list[Path]:
    targets: set[Path] = set()
    targets.update(repo_root.glob("scripts/**/*.sh"))
    targets.update(repo_root.glob("nix/**/*.nix"))
    targets.update(repo_root.glob("packages/**/Justfile"))
    targets.update(repo_root.glob("packages/**/justfile"))
    targets.update(repo_root.glob("*.yaml"))
    targets.update(repo_root.glob("*.yml"))
    targets.update((repo_root / ".github" / "workflows").glob("*.yaml"))
    targets.update((repo_root / ".github" / "workflows").glob("*.yml"))
    targets.update((repo_root / ".github" / "actions").glob("**/*.yaml"))
    targets.update((repo_root / ".github" / "actions").glob("**/*.yml"))

    root_justfile = repo_root / "justfile"
    if root_justfile.exists():
        targets.add(root_justfile)

    return sorted(path for path in targets if path.is_file())


def _is_comment_line(line: str) -> bool:
    return line.lstrip().startswith("#")


def test_no_inline_python_exec_patterns_in_shell_workflow_or_task_files() -> None:
    repo_root = Path(__file__).resolve().parents[1]
    violations: list[str] = []

    for path in _iter_target_files(repo_root):
        rel_path = path.relative_to(repo_root)
        for index, raw_line in enumerate(
            path.read_text(encoding="utf-8", errors="ignore").splitlines(), start=1
        ):
            line = raw_line.rstrip("\n")
            if not line.strip() or _is_comment_line(line):
                continue
            for pattern in FORBIDDEN_PATTERNS:
                if pattern.search(line):
                    violations.append(f"{rel_path}:{index}: {line.strip()}")
                    break

    assert not violations, "inline Python execution pattern found:\n" + "\n".join(violations)
