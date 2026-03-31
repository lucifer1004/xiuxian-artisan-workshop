import importlib
import shutil
import sys
from pathlib import Path

import pytest

from xiuxian_foundation.api.decorators import normalize_tool_result


def _load_search_module():
    scripts_dir = Path(__file__).parent.parent / "scripts"
    if str(scripts_dir) not in sys.path:
        sys.path.insert(0, str(scripts_dir))
    sys.modules.pop("search", None)
    return importlib.import_module("search")


def _load_mutation_module():
    scripts_dir = Path(__file__).parent.parent / "scripts"
    if str(scripts_dir) not in sys.path:
        sys.path.insert(0, str(scripts_dir))
    sys.modules.pop("mutation", None)
    return importlib.import_module("mutation")


def _unwrap_payload(result: object) -> dict:
    normalized = normalize_tool_result(result)
    return (
        normalized
        if "tool" in normalized
        else __import__("json").loads(normalized["content"][0]["text"])
    )


class TestAdvancedToolsPatternMode:
    """Unit tests for literal-vs-regex mode selection."""

    def test_fixed_strings_enabled_for_literal_pattern(self):
        module = _load_search_module()
        _should_use_fixed_strings = module._should_use_fixed_strings

        assert _should_use_fixed_strings("Hard Constraints") is True
        assert _should_use_fixed_strings("README.md") is False

    def test_fixed_strings_disabled_for_regex_pattern(self):
        module = _load_search_module()
        _should_use_fixed_strings = module._should_use_fixed_strings

        assert _should_use_fixed_strings(r"Hard\\s+Constraints") is False
        assert _should_use_fixed_strings("foo(bar)") is False

    def test_resolve_search_root_supports_relative_path(self, tmp_path: Path):
        module = _load_search_module()
        _resolve_search_root = module._resolve_search_root

        fixture_dir = tmp_path / "fixtures"
        fixture_dir.mkdir(parents=True, exist_ok=True)

        resolved = _resolve_search_root(str(tmp_path), "fixtures")
        assert resolved == str(fixture_dir.resolve())

    def test_resolve_search_root_rejects_missing_path(self, tmp_path: Path):
        module = _load_search_module()
        _resolve_search_root = module._resolve_search_root

        with pytest.raises(ValueError):
            _resolve_search_root(str(tmp_path), "missing")

    def test_resolve_exec_uses_cached_which_results(self, monkeypatch: pytest.MonkeyPatch):
        module = _load_search_module()
        _resolve_exec = module._resolve_exec
        _which_cached = module._which_cached

        _which_cached.cache_clear()
        calls: list[str] = []

        def _fake_which(name: str) -> str | None:
            calls.append(name)
            if name == "fd":
                return "/usr/bin/fd"
            return None

        monkeypatch.setattr(module.shutil, "which", _fake_which)

        first = _resolve_exec("fd", "fdfind")
        second = _resolve_exec("fd", "fdfind")

        assert first == "/usr/bin/fd"
        assert second == "/usr/bin/fd"
        assert calls == ["fd"]

    def test_parse_vimgrep_line_returns_normalized_match(self):
        module = _load_search_module()
        _parse_vimgrep_line = module._parse_vimgrep_line

        line = "docs/guide.md:42:7:Hard Constraints section"
        parsed = _parse_vimgrep_line(line)

        assert parsed is not None
        assert parsed["file"] == "docs/guide.md"
        assert parsed["line"] == 42
        assert parsed["content"] == "Hard Constraints section"

    def test_parse_vimgrep_line_rejects_invalid_payload(self):
        module = _load_search_module()
        _parse_vimgrep_line = module._parse_vimgrep_line

        assert _parse_vimgrep_line("not-a-vimgrep-line") is None

    def test_can_use_python_filename_fast_path_requires_scoped_literal_query(self):
        module = _load_search_module()
        _can_use_python_filename_fast_path = module._can_use_python_filename_fast_path

        assert _can_use_python_filename_fast_path(
            pattern="benchmark_note",
            exclude=None,
            resolved_search_root="/tmp",
        )
        assert not _can_use_python_filename_fast_path(
            pattern="test_*.py",
            exclude=None,
            resolved_search_root="/tmp",
        )
        assert not _can_use_python_filename_fast_path(
            pattern="benchmark_note",
            exclude="target",
            resolved_search_root="/tmp",
        )
        assert not _can_use_python_filename_fast_path(
            pattern="benchmark_note",
            exclude=None,
            resolved_search_root=None,
        )

    def test_python_fast_find_files_filters_hidden_and_extension(self, tmp_path: Path):
        module = _load_search_module()
        _python_fast_find_files = module._python_fast_find_files

        docs_dir = tmp_path / "docs"
        hidden_dir = docs_dir / ".private"
        docs_dir.mkdir(parents=True, exist_ok=True)
        hidden_dir.mkdir(parents=True, exist_ok=True)

        (docs_dir / "benchmark_note.md").write_text("visible")
        (hidden_dir / "benchmark_note.md").write_text("hidden")
        (docs_dir / "benchmark_note.txt").write_text("text")

        files = _python_fast_find_files(
            project_root=str(tmp_path),
            search_root=str(docs_dir),
            pattern="benchmark_note",
            extension="md",
            max_results=100,
        )

        assert files == ["docs/benchmark_note.md"]

    def test_smart_search_reuses_cached_result_for_identical_query(
        self,
        monkeypatch: pytest.MonkeyPatch,
        tmp_path: Path,
    ) -> None:
        module = _load_search_module()
        clear_smart_search_cache = module.clear_smart_search_cache
        smart_search = module.smart_search

        calls = {"count": 0}

        def _fake_run_rg_with_retry(cmd: list[str], root: str, max_retries: int = 2):
            del cmd, root, max_retries
            calls["count"] += 1
            return "docs/guide.md:42:7:Hard Constraints section\n", "", 0

        monkeypatch.setattr(module, "_run_rg_with_retry", _fake_run_rg_with_retry)

        clear_smart_search_cache()
        try:
            first = smart_search(
                pattern="Hard Constraints",
                file_globs="*.md",
                case_sensitive=True,
                context_lines=0,
                project_root=tmp_path,
            )
            second = smart_search(
                pattern="Hard Constraints",
                file_globs="*.md",
                case_sensitive=True,
                context_lines=0,
                project_root=tmp_path,
            )
        finally:
            clear_smart_search_cache()

        assert first == second
        assert calls["count"] == 1


class TestAdvancedToolsModular:
    """Retained modular tests for advanced_tools scripts."""

    def test_smart_search(self, tmp_path: Path):
        module = _load_search_module()
        target = tmp_path / "sample.py"
        target.write_text("import pytest\n")

        payload = _unwrap_payload(
            module.smart_search(
                pattern="import pytest",
                file_globs="*.py",
                search_root=str(tmp_path),
                project_root=tmp_path,
            )
        )

        assert payload["tool"] == "ripgrep"
        assert isinstance(payload["matches"], list)

    def test_smart_find(self, tmp_path: Path):
        if not shutil.which("fd"):
            pytest.skip("fd command not installed")

        module = _load_search_module()
        (tmp_path / "test_alpha.py").write_text("x = 1\n")

        payload = _unwrap_payload(
            module.smart_find(
                pattern="test_*.py",
                extension="py",
                search_root=str(tmp_path),
                project_root=tmp_path,
            )
        )

        assert payload["tool"] == "fd"
        assert isinstance(payload["files"], list)

    def test_regex_replace(self, tmp_path: Path):
        if not shutil.which("sed"):
            pytest.skip("sed command not installed")

        module = _load_mutation_module()
        test_file = tmp_path / "test_regex_replace_temp.txt"
        test_file.write_text("Hello World")

        result = _unwrap_payload(
            module.regex_replace(
                file_path=str(test_file),
                pattern="World",
                replacement="Modular",
                project_root=tmp_path,
            )
        )

        assert result["success"] is True
        assert test_file.read_text().strip() == "Hello Modular"

    def test_batch_replace_dry_run(self, tmp_path: Path):
        module = _load_mutation_module()
        test_dir = tmp_path / "test_batch_temp"
        test_dir.mkdir(exist_ok=True)
        (test_dir / "file1.py").write_text("old_val = 1")
        (test_dir / "file2.py").write_text("old_val = 2")

        payload = _unwrap_payload(
            module.batch_replace(
                pattern="old_val",
                replacement="new_val",
                file_glob="test_batch_temp/*.py",
                dry_run=True,
                project_root=tmp_path,
            )
        )

        assert payload["success"] is True
        assert payload["mode"] == "Dry-Run"
