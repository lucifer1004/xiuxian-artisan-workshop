from __future__ import annotations

from pathlib import Path


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _read(path: str) -> str:
    return (_repo_root() / path).read_text(encoding="utf-8")


def _just_recipe_block(recipe_name: str) -> str:
    justfile = _read("justfile")
    marker = f"{recipe_name}:\n"
    start = justfile.index(marker)
    next_group = justfile.find("\n[group('", start + len(marker))
    if next_group == -1:
        return justfile[start:]
    return justfile[start:next_group]


def test_agent_channel_startup_scripts_do_not_default_removed_xiuxian_llm_features() -> None:
    for path in (
        "scripts/channel/agent-channel-webhook.sh",
        "scripts/channel/agent-channel-polling.sh",
    ):
        content = _read(path)
        assert "XIUXIAN_DAOCHANG_CARGO_FEATURES" in content
        assert "mistral-accel" not in content
        assert "xiuxian-llm/vision-dots" not in content


def test_agent_channel_discord_ingress_recipe_does_not_default_removed_xiuxian_llm_features() -> (
    None
):
    recipe = _just_recipe_block("agent-channel-discord-ingress")
    assert "XIUXIAN_DAOCHANG_CARGO_FEATURES" in recipe
    assert "mistral-accel" not in recipe
    assert "xiuxian-llm/vision-dots" not in recipe


def test_local_model_safe_script_no_longer_uses_removed_mistral_switches() -> None:
    content = _read("scripts/rust/test_local_models_safe.sh")
    assert "mistral-accel" not in content
    assert "--mistral-sdk-only" not in content


def test_webhook_startup_script_no_longer_uses_removed_mistral_warmup_switch() -> None:
    content = _read("scripts/channel/agent-channel-webhook.sh")
    assert "--mistral-sdk-only" not in content


def test_vision_command_surfaces_do_not_use_removed_xiuxian_llm_features() -> None:
    justfile = _read("justfile")
    heavy_lane = _read("scripts/rust/test_vision_heavy_lane.sh")

    assert "vision-dots-metal" not in justfile
    assert "vision-dots-cuda" not in justfile
    assert "vision-dots" not in heavy_lane
    assert "--features" not in heavy_lane


def test_backend_role_contract_script_no_longer_references_removed_mistral_aliases() -> None:
    content = _read("scripts/rust/xiuxian_daochang_backend_role_contracts.sh")
    assert "mistral_sdk" not in content
    assert "gateway_preserves_configured_http_embedding_backend" in content
    assert "test -p xiuxian-llm --test unit_test \\" in content


def test_embedding_role_perf_smoke_scripts_use_active_backend_roles() -> None:
    python_script = _read("scripts/rust/xiuxian_daochang_embedding_role_perf_smoke.py")
    shell_script = _read("scripts/rust/xiuxian_daochang_embedding_role_perf_smoke.sh")

    assert "mistral_sdk" not in python_script
    assert 'name="openai_http"' in python_script
    assert 'backend = "openai_http"' in python_script
    assert 'embedding_backend = "openai_http"' in python_script
    assert "litellm_rs + openai_http" in shell_script
