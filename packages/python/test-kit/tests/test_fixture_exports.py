"""Tests for package-level fixture exports."""

from __future__ import annotations

import xiuxian_test_kit.fixtures as fixtures


def test_fixture_package_exports_git_fixtures() -> None:
    assert hasattr(fixtures, "git_repo")
    assert hasattr(fixtures, "temp_git_repo")


def test_fixture_package_does_not_auto_export_live_wire_fixtures() -> None:
    assert not hasattr(fixtures, "live_wire_mock_server")
    assert not hasattr(fixtures, "registered_live_wire_server")


def test_fixture_package_does_not_export_removed_scanner_or_watcher_helpers() -> None:
    assert not hasattr(fixtures, "SkillTestSuite")
    assert not hasattr(fixtures, "skill_test_suite")
    assert not hasattr(fixtures, "skill_directory")
    assert not hasattr(fixtures, "multi_skill_directory")
    assert not hasattr(fixtures, "parametrize_skills")
    assert not hasattr(fixtures, "mock_watcher_indexer")
    assert not hasattr(fixtures, "mock_watcher_indexer_with_count")
    assert not hasattr(fixtures, "WatcherTestHelper")
    assert not hasattr(fixtures, "watcher_test_helper")


def test_fixture_package_does_not_export_removed_knowledge_store_helper() -> None:
    assert not hasattr(fixtures, "mock_knowledge_graph_store")


def test_fixture_package_does_not_export_removed_skill_execution_helpers() -> None:
    assert not hasattr(fixtures, "skill_tester")
    assert not hasattr(fixtures, "SkillResult")
    assert not hasattr(fixtures, "SkillTester")
    assert not hasattr(fixtures, "mcp_tester")
    assert not hasattr(fixtures, "skills_root")
    assert not hasattr(fixtures, "SkillTestBuilder")
