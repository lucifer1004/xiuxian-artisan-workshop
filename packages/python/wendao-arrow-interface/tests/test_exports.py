from __future__ import annotations

from inspect import isclass
from pathlib import Path
import tomllib

import wendao_arrow_interface as interface


def _pyproject_version() -> str:
    pyproject = Path(__file__).resolve().parents[1] / "pyproject.toml"
    return tomllib.loads(pyproject.read_text(encoding="utf-8"))["project"]["version"]


def _pyproject_data() -> dict[str, object]:
    pyproject = Path(__file__).resolve().parents[1] / "pyproject.toml"
    return tomllib.loads(pyproject.read_text(encoding="utf-8"))


def test_public_exports_resolve_from_package_root() -> None:
    exported_names = interface.__all__

    assert exported_names
    assert len(exported_names) == len(set(exported_names))

    for name in exported_names:
        assert hasattr(interface, name), name


def test_public_exports_include_core_interface_surface() -> None:
    assert "__version__" in interface.__all__
    assert "WendaoArrowSession" in interface.__all__
    assert "WendaoArrowResult" in interface.__all__
    assert "WendaoArrowScriptedClient" in interface.__all__
    assert "WendaoArrowCall" in interface.__all__
    assert "WendaoAttachmentSearchRequest" in interface.__all__
    assert "WendaoAttachmentSearchResultRow" in interface.__all__
    assert "SEARCH_ATTACHMENTS_ROUTE" in interface.__all__
    assert "ArrowTableParser" in interface.__all__
    assert "RowsAnalyzer" in interface.__all__
    assert "connect" in interface.__all__
    assert "attachment_search_request" in interface.__all__
    assert "attachment_search_metadata" in interface.__all__
    assert "parse_attachment_search_rows" in interface.__all__
    assert "repo_search_metadata" in interface.__all__
    assert "repo_search_request" in interface.__all__
    assert "parse_repo_search_rows" in interface.__all__
    assert "parse_rerank_response_rows" in interface.__all__
    assert "rerank_request_metadata" in interface.__all__
    assert "PolarsFrameParser" not in interface.__all__
    assert "PolarsFrameAnalyzer" not in interface.__all__


def test_public_exports_preserve_expected_symbol_kinds() -> None:
    assert isclass(interface.WendaoArrowSession)
    assert isclass(interface.WendaoArrowResult)
    assert isclass(interface.WendaoArrowScriptedClient)
    assert isclass(interface.WendaoArrowCall)
    assert isclass(interface.WendaoAttachmentSearchRequest)
    assert isclass(interface.WendaoAttachmentSearchResultRow)
    assert isclass(interface.WendaoTransportClient)
    assert isclass(interface.WendaoTransportConfig)
    assert isclass(interface.WendaoTransportEndpoint)
    assert isclass(interface.WendaoFlightRouteQuery)
    assert isclass(interface.WendaoRepoSearchRequest)
    assert isclass(interface.WendaoRepoSearchResultRow)
    assert isclass(interface.WendaoRerankRequestRow)
    assert isclass(interface.WendaoRerankResultRow)

    assert callable(interface.connect)
    assert callable(interface.attachment_search_request)
    assert callable(interface.attachment_search_metadata)
    assert callable(interface.parse_attachment_search_rows)
    assert callable(interface.repo_search_metadata)
    assert callable(interface.repo_search_request)
    assert callable(interface.parse_repo_search_rows)
    assert callable(interface.parse_rerank_response_rows)
    assert callable(interface.rerank_request_metadata)
    assert callable(interface.WendaoArrowResult.from_rows)
    assert callable(interface.WendaoArrowResult.from_query_rows)
    assert callable(interface.WendaoArrowResult.from_exchange_rows)
    assert callable(interface.WendaoArrowResult.from_attachment_search_result_rows)
    assert callable(interface.WendaoArrowResult.from_repo_search_result_rows)
    assert callable(interface.WendaoArrowResult.from_rerank_response_rows)
    assert callable(interface.WendaoArrowSession.for_query_testing)
    assert callable(interface.WendaoArrowSession.for_exchange_testing)
    assert callable(interface.WendaoArrowSession.for_attachment_search_testing)
    assert callable(interface.WendaoArrowSession.for_repo_search_testing)
    assert callable(interface.WendaoArrowSession.for_rerank_response_testing)
    assert callable(interface.WendaoArrowScriptedClient.for_query_route)
    assert callable(interface.WendaoArrowScriptedClient.for_exchange_route)
    assert callable(interface.WendaoArrowScriptedClient.for_attachment_search_rows)
    assert callable(interface.WendaoArrowScriptedClient.for_repo_search_rows)
    assert callable(interface.WendaoArrowScriptedClient.for_rerank_response_rows)
    assert callable(interface.WendaoArrowScriptedClient.add_query_response)
    assert callable(interface.WendaoArrowScriptedClient.add_exchange_response)
    assert callable(interface.WendaoArrowScriptedClient.add_attachment_search_response)
    assert callable(interface.WendaoArrowScriptedClient.add_repo_search_response)
    assert callable(interface.WendaoArrowScriptedClient.add_rerank_response)
    assert isinstance(
        interface.WendaoArrowCall(operation="query", route="/").effective_metadata, dict
    )
    assert callable(interface.WendaoArrowCall.derived_metadata)
    assert callable(interface.WendaoArrowCall.metadata_matches_contract)
    assert callable(interface.WendaoArrowCall.assert_metadata_matches_contract)
    assert not hasattr(interface.WendaoArrowResult, "parse_dataframe")
    assert not hasattr(interface.WendaoArrowResult, "analyze_dataframe")


def test_package_root_exports_version_matching_pyproject() -> None:
    assert interface.__version__ == "0.2.4"
    assert interface.__version__ == _pyproject_version()


def test_pyproject_marks_polars_as_optional_adapter_dependency() -> None:
    pyproject = _pyproject_data()
    project = pyproject["project"]

    assert "polars>=1.0.0" not in project["dependencies"]
    assert project["optional-dependencies"]["polars"] == ["polars>=1.0.0"]
