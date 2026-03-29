"""Unit tests for scripts/llm_fingerprint_audit.py."""

from __future__ import annotations

import importlib.util
import sys

from xiuxian_wendao_py.compat.runtime import get_project_root


def _load_module():
    root = get_project_root()
    script_path = root / "scripts" / "llm_fingerprint_audit.py"
    spec = importlib.util.spec_from_file_location("xiuxian_llm_fingerprint_audit", script_path)
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def test_load_codex_provider_reads_named_provider(tmp_path) -> None:
    module = _load_module()
    config_path = tmp_path / "config.toml"
    config_path.write_text(
        "\n".join(
            [
                'model_provider = "crs"',
                'model = "gpt-5.4"',
                "",
                "[model_providers.crs]",
                'base_url = "https://example.test/openai"',
                'wire_api = "responses"',
                'env_key = "CRS_OAI_KEY"',
            ]
        ),
        encoding="utf-8",
    )

    provider = module.load_codex_provider(config_path, None, None)

    assert provider.name == "crs"
    assert provider.model == "gpt-5.4"
    assert provider.base_url == "https://example.test/openai"
    assert provider.api_key_env == "CRS_OAI_KEY"
    assert provider.wire_api == "responses"


def test_normalize_openai_compatible_base_appends_v1() -> None:
    module = _load_module()
    assert module.normalize_openai_compatible_base("https://example.test/openai") == (
        "https://example.test/openai/v1"
    )
    assert module.normalize_openai_compatible_base("https://example.test/v1") == (
        "https://example.test/v1"
    )


def test_extract_responses_stream_reply_supports_completed_event() -> None:
    module = _load_module()
    stream_text = "\n".join(
        [
            'data: {"type":"response.completed","response":{"output":[{"content":[{"type":"output_text","text":"hello world"}]}]}}',
            "data: [DONE]",
        ]
    )
    assert module.extract_responses_stream_reply(stream_text) == "hello world"


def test_extract_responses_instruction_echo_chars_detects_leak() -> None:
    module = _load_module()
    stream_text = "\n".join(
        [
            'data: {"type":"response.completed","response":{"instructions":"secret system prompt","output":[{"content":[{"type":"output_text","text":"hello world"}]}]}}',
            "data: [DONE]",
        ]
    )

    assert module.extract_responses_instruction_echo_chars(stream_text) == len(
        "secret system prompt"
    )


def test_baseline_provider_from_args_supports_custom_proxy() -> None:
    module = _load_module()

    class Args:
        baseline_base_url = "https://proxy.example.test/openai"
        baseline_api_key_env = "PROXY_KEY"
        baseline_wire_api = "responses"
        baseline_name = "trusted_proxy"

    provider = module.baseline_provider_from_args("gpt-4o", Args())

    assert provider.name == "trusted_proxy"
    assert provider.model == "gpt-4o"
    assert provider.base_url == "https://proxy.example.test/openai"
    assert provider.api_key_env == "PROXY_KEY"
    assert provider.wire_api == "responses"


def test_extract_chat_stream_reply_supports_delta_events() -> None:
    module = _load_module()
    stream_text = "\n".join(
        [
            'data: {"choices":[{"delta":{"content":"hello "}}]}',
            'data: {"choices":[{"delta":{"content":"world"}}]}',
            "data: [DONE]",
        ]
    )

    assert module.extract_chat_stream_reply(stream_text) == "hello world"


def test_embedding_config_from_args_supports_http_batch() -> None:
    module = _load_module()

    class Args:
        embedding_backend = "http_batch"
        embedding_base_url = "http://127.0.0.1:18092"
        embedding_api_key_env = "IGNORED"
        embedding_model = "local-default"

    config = module.embedding_config_from_args(Args())

    assert config == {
        "backend": "http_batch",
        "base_url": "http://127.0.0.1:18092",
        "api_key_env": "IGNORED",
        "model": "local-default",
    }


def test_request_backend_arg_defaults_to_urllib() -> None:
    module = _load_module()
    parser = module.parse_args
    old_argv = sys.argv
    sys.argv = ["llm_fingerprint_audit.py"]
    try:
        args = parser()
    finally:
        sys.argv = old_argv

    assert args.request_backend == "urllib"
    assert args.request_retries == 3


def test_compute_probe_metrics_reports_ratio() -> None:
    module = _load_module()
    baseline_vectors = [[1.0, 0.0], [0.99, 0.01], [0.98, 0.02]]
    suspect_vector = [0.0, 1.0]

    metrics = module.compute_probe_metrics(baseline_vectors, suspect_vector)

    assert metrics["baseline_dispersion"] >= 0.0
    assert metrics["suspect_distance"] > metrics["baseline_dispersion"]
    assert metrics["ratio"] > 1.2
    assert module.classify_ratio(metrics["ratio"], 1.2) == "mismatch"
