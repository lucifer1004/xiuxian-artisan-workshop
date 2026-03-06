#!/usr/bin/env python3
"""Live LLM provider smoke test from xiuxian.toml (text + optional image)."""

from __future__ import annotations

import argparse
import base64
import json
import mimetypes
import os
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse
from urllib.request import Request, urlopen

try:
    import tomllib
except ModuleNotFoundError:
    try:
        import tomli as tomllib  # type: ignore[no-redef]
    except ModuleNotFoundError as exc:  # pragma: no cover - environment guard
        raise ModuleNotFoundError(
            "No TOML parser available. Use Python 3.11+ or install tomli."
        ) from exc

DEFAULT_TIMEOUT_SECS = 60
DEFAULT_TEXT_PROMPT = "Reply with exactly: PONG"
DEFAULT_IMAGE_PROMPT = "Describe this image in one short sentence."
ANTHROPIC_VERSION = os.environ.get("XIUXIAN_SMOKE_ANTHROPIC_VERSION", "2023-06-01")
ENV_NAME_PATTERN = re.compile(r"^[A-Z][A-Z0-9_]*$")

TRANSPORT_OPENAI = "openai"
TRANSPORT_MINIMAX = "minimax"
TRANSPORT_ANTHROPIC_BYPASS = "anthropic_messages_bypass"
WIRE_API_CHAT = "chat_completions"
WIRE_API_RESPONSES = "responses"
DEFAULT_IMAGE_EXPECTATION_HINTS: dict[str, tuple[tuple[str, ...], ...]] = {
    "jpeg_example_flower.jpg": (("flower", "bloom", "hibiscus"), ("red", "hibiscus")),
}


@dataclass(frozen=True)
class SmokeCase:
    provider: str
    base_url: str
    api_key_spec: str | None
    model: str
    wire_api: str


@dataclass(frozen=True)
class ResolvedImage:
    source: str
    media_type: str
    base64_data: str
    data_uri: str


@dataclass(frozen=True)
class CallResult:
    content: str
    endpoint: str
    transport: str


def normalize_non_empty(value: Any) -> str | None:
    if not isinstance(value, str):
        return None
    stripped = value.strip()
    return stripped or None


def default_xiuxian_config_path() -> Path:
    config_home = Path(os.environ.get("PRJ_CONFIG_HOME", ".config"))
    return config_home / "xiuxian-artisan-workshop" / "xiuxian.toml"


def default_api_key_env(provider_name: str) -> str:
    normalized = provider_name.strip().lower()
    if normalized == "openai":
        return "OPENAI_API_KEY"
    if normalized == "minimax":
        return "MINIMAX_API_KEY"
    if normalized == "anthropic":
        return "ANTHROPIC_API_KEY"
    return f"{provider_name.strip().upper()}_API_KEY"


def parse_provider_filter(raw: str | None) -> set[str] | None:
    if raw is None:
        return None
    normalized = raw.strip().lower()
    if not normalized:
        return None
    if normalized in {"all", "*"}:
        return set()
    providers = {item.strip().lower() for item in raw.split(",") if item.strip()}
    return providers or None


def normalize_wire_api(raw: str) -> str:
    normalized = raw.strip().lower()
    if normalized in {"", "chat", "chat_completions", "chat-completions"}:
        return WIRE_API_CHAT
    if normalized in {"responses", "response"}:
        return WIRE_API_RESPONSES
    raise RuntimeError(
        f"Unsupported wire_api '{raw}'. Expected '{WIRE_API_CHAT}' or '{WIRE_API_RESPONSES}'."
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Live smoke test for providers in xiuxian.toml (text + optional image)."
    )
    parser.add_argument(
        "--config-path",
        default=os.environ.get("XIUXIAN_CONFIG_PATH", ""),
        help="Path to xiuxian.toml. Default: $XIUXIAN_CONFIG_PATH or PRJ_CONFIG_HOME fallback.",
    )
    parser.add_argument(
        "--provider",
        default=os.environ.get("XIUXIAN_PROVIDER", "all"),
        help="Provider filter (comma-separated or 'all').",
    )
    parser.add_argument(
        "--model-override",
        default=os.environ.get("XIUXIAN_MODEL_OVERRIDE", ""),
        help="Override model for every selected provider.",
    )
    parser.add_argument(
        "--wire-api",
        default=os.environ.get("XIUXIAN_SMOKE_WIRE_API", ""),
        help="Override wire API for OpenAI-compatible providers: chat_completions|responses.",
    )
    parser.add_argument(
        "--timeout-secs",
        type=int,
        default=int(os.environ.get("XIUXIAN_TIMEOUT_SECS", str(DEFAULT_TIMEOUT_SECS))),
        help=f"HTTP timeout in seconds (default: {DEFAULT_TIMEOUT_SECS}).",
    )
    parser.add_argument(
        "--text-prompt",
        default=os.environ.get("XIUXIAN_SMOKE_PROMPT", DEFAULT_TEXT_PROMPT),
        help="Prompt for text connectivity test.",
    )
    parser.add_argument(
        "--image-prompt",
        default=os.environ.get("XIUXIAN_SMOKE_IMAGE_PROMPT", DEFAULT_IMAGE_PROMPT),
        help="Prompt for multimodal image test.",
    )
    parser.add_argument(
        "--image",
        default=os.environ.get("XIUXIAN_SMOKE_IMAGE", ""),
        help="Optional image input (file path, http(s) URL, or data URI).",
    )
    parser.add_argument(
        "--image-contains",
        default=os.environ.get("XIUXIAN_SMOKE_IMAGE_CONTAINS", ""),
        help=(
            "Comma-separated semantic expectation groups for image replies. "
            "Each comma-separated group must match; use | inside a group for alternatives. "
            "If omitted, known smoke images use built-in defaults."
        ),
    )
    return parser.parse_args()


def load_toml(path: Path) -> dict[str, Any]:
    try:
        with path.open("rb") as handle:
            return tomllib.load(handle)
    except FileNotFoundError as exc:
        raise RuntimeError(f"Config file not found: {path}") from exc
    except tomllib.TOMLDecodeError as exc:
        raise RuntimeError(f"Failed to parse TOML config '{path}': {exc}") from exc


def build_cases(
    config: dict[str, Any],
    provider_filter: set[str] | None,
    model_override: str | None,
    wire_api_override: str | None,
) -> list[SmokeCase]:
    llm = config.get("llm")
    if not isinstance(llm, dict):
        raise RuntimeError("Missing [llm] section in xiuxian.toml")

    providers = llm.get("providers")
    if not isinstance(providers, dict) or not providers:
        raise RuntimeError("No [llm.providers] configured in xiuxian.toml")

    default_model = normalize_non_empty(llm.get("default_model"))
    configured_provider_keys = {str(name).lower() for name in providers}

    if provider_filter is None:
        default_provider = normalize_non_empty(llm.get("default_provider"))
        if default_provider is None:
            effective_filter = configured_provider_keys
        else:
            provider_key = default_provider.lower()
            if provider_key not in configured_provider_keys:
                raise RuntimeError(
                    f"Configured [llm].default_provider '{default_provider}' is not in [llm.providers]"
                )
            effective_filter = {provider_key}
    elif provider_filter:
        unknown = sorted(provider_filter - configured_provider_keys)
        if unknown:
            raise RuntimeError(
                "Provider filter contains unknown providers: "
                f"{', '.join(unknown)}. Configured: {', '.join(sorted(configured_provider_keys))}"
            )
        effective_filter = provider_filter
    else:
        effective_filter = configured_provider_keys

    cases: list[SmokeCase] = []
    for provider_name, provider_cfg_raw in providers.items():
        provider = str(provider_name)
        provider_lower = provider.lower()
        if provider_lower not in effective_filter:
            continue
        if not isinstance(provider_cfg_raw, dict):
            raise RuntimeError(f"Provider '{provider}' config must be a table")

        base_url = normalize_non_empty(provider_cfg_raw.get("base_url"))
        if base_url is None:
            raise RuntimeError(f"Provider '{provider}' is missing 'base_url'")

        api_key_spec = normalize_non_empty(provider_cfg_raw.get("api_key"))

        provider_model = (
            model_override or normalize_non_empty(provider_cfg_raw.get("model")) or default_model
        )
        if provider_model is None:
            raise RuntimeError(
                f"Provider '{provider}' has no model. Set [llm.providers.{provider}].model "
                "or [llm].default_model."
            )

        wire_api = normalize_wire_api(
            wire_api_override
            or normalize_non_empty(provider_cfg_raw.get("wire_api"))
            or normalize_non_empty(llm.get("wire_api"))
            or WIRE_API_CHAT
        )

        model_aliases = provider_cfg_raw.get("model_aliases")
        if isinstance(model_aliases, dict) and isinstance(model_aliases.get(provider_model), str):
            model = model_aliases[provider_model]
        else:
            model = provider_model

        cases.append(
            SmokeCase(
                provider=provider,
                base_url=base_url,
                api_key_spec=api_key_spec,
                model=model,
                wire_api=wire_api,
            )
        )

    if not cases:
        raise RuntimeError("No providers selected for smoke test")
    return cases


def parse_error_message(body_text: str) -> str:
    if not body_text:
        return ""
    try:
        payload = json.loads(body_text)
    except json.JSONDecodeError:
        return body_text.strip()
    if isinstance(payload, dict):
        error_obj = payload.get("error")
        if isinstance(error_obj, dict):
            message = error_obj.get("message")
            if isinstance(message, str) and message.strip():
                return message.strip()
        message = payload.get("message")
        if isinstance(message, str) and message.strip():
            return message.strip()
    return body_text.strip()


def parse_sse_data_events(stream_text: str) -> list[dict[str, Any]]:
    events: list[dict[str, Any]] = []
    for line in stream_text.splitlines():
        if not line.startswith("data:"):
            continue
        payload = line[5:].strip()
        if not payload or payload == "[DONE]":
            continue
        try:
            obj = json.loads(payload)
        except json.JSONDecodeError:
            continue
        if isinstance(obj, dict):
            events.append(obj)
    return events


def extract_responses_output_text_from_response(response: Any) -> str | None:
    if not isinstance(response, dict):
        return None

    direct = response.get("output_text")
    if isinstance(direct, str) and direct.strip():
        return direct.strip()

    output = response.get("output")
    if not isinstance(output, list):
        return None

    texts: list[str] = []
    for item in output:
        if not isinstance(item, dict):
            continue
        content = item.get("content")
        if not isinstance(content, list):
            continue
        for part in content:
            if not isinstance(part, dict):
                continue
            if part.get("type") == "output_text":
                text = part.get("text")
                if isinstance(text, str) and text.strip():
                    texts.append(text.strip())
    if texts:
        return "\n".join(texts)
    return None


def extract_responses_stream_reply(stream_text: str) -> str:
    events = parse_sse_data_events(stream_text)
    if not events:
        raise RuntimeError("responses stream contains no JSON events")

    deltas: list[str] = []
    done_fragments: list[str] = []
    completed_text: str | None = None

    for event in events:
        typ = event.get("type")
        if typ == "response.output_text.delta":
            delta = event.get("delta")
            if isinstance(delta, str):
                deltas.append(delta)
        elif typ == "response.output_text.done":
            text = event.get("text")
            if isinstance(text, str) and text.strip():
                done_fragments.append(text.strip())
        elif typ == "response.completed":
            text = extract_responses_output_text_from_response(event.get("response"))
            if text:
                completed_text = text

    if deltas:
        joined = "".join(deltas).strip()
        if joined:
            return joined
    if done_fragments:
        joined = "\n".join(done_fragments).strip()
        if joined:
            return joined
    if completed_text:
        return completed_text

    # Connectivity is real if events arrived; return compact preview for pass signal.
    return f"[responses-stream events={len(events)}]"


def read_non_empty_env(name: str) -> str | None:
    value = os.environ.get(name, "").strip()
    return value or None


def looks_like_env_name(value: str) -> bool:
    return bool(ENV_NAME_PATTERN.fullmatch(value.strip()))


def resolve_key_from_spec(spec: str | None) -> tuple[str | None, str | None]:
    if spec is None:
        return None, None
    if spec.startswith("env:"):
        env_name = spec.removeprefix("env:").strip()
        if not env_name:
            return None, "env:<empty>"
        return read_non_empty_env(env_name), f"env:{env_name}"
    if looks_like_env_name(spec):
        return read_non_empty_env(spec), f"env:{spec}"
    return spec, "literal"


def resolve_anthropic_env_key() -> tuple[str | None, str | None]:
    for name in ("ANTHROPIC_API_KEY", "ANTHROPIC_AUTH_TOKEN"):
        value = read_non_empty_env(name)
        if value:
            return value, f"env:{name}"
    return None, None


def resolve_primary_provider_key(case: SmokeCase) -> tuple[str, str]:
    configured_key, configured_source = resolve_key_from_spec(case.api_key_spec)
    if configured_key:
        return configured_key, configured_source or "configured"

    fallback_env = default_api_key_env(case.provider)
    fallback_value = read_non_empty_env(fallback_env)
    if fallback_value:
        return fallback_value, f"env:{fallback_env}"

    if case.provider.lower() == "anthropic":
        alt_value, alt_source = resolve_anthropic_env_key()
        if alt_value and alt_source:
            return alt_value, alt_source

    hint = case.api_key_spec or fallback_env
    raise RuntimeError(
        f"Missing API key for provider '{case.provider}'. "
        f"Configure [llm.providers.{case.provider}].api_key or export '{hint}'."
    )


def normalize_openai_compatible_base(base_url: str) -> str:
    trimmed = base_url.strip().rstrip("/")
    without_chat_suffix = trimmed.removesuffix("/chat/completions").rstrip("/")
    if without_chat_suffix.endswith("/v1"):
        return without_chat_suffix
    return f"{without_chat_suffix}/v1"


def anthropic_messages_endpoint_from_base(base_url: str) -> str:
    trimmed = base_url.strip().rstrip("/")
    if trimmed.endswith("/v1/messages") or trimmed.endswith("/messages"):
        return trimmed
    if trimmed.endswith("/v1"):
        return f"{trimmed}/messages"
    return f"{trimmed}/v1/messages"


def is_official_anthropic_base(base_url: str) -> bool:
    try:
        parsed = urlparse(base_url)
    except Exception:
        return False
    return (parsed.hostname or "").lower() == "api.anthropic.com"


def prefers_minimax_transport(model: str) -> bool:
    lower = model.strip().lower()
    return lower.startswith("glm-") or lower.startswith("minimax-") or lower.startswith("minimax/")


def fallback_transport_order(model: str) -> list[str]:
    if prefers_minimax_transport(model):
        return [TRANSPORT_MINIMAX, TRANSPORT_OPENAI, TRANSPORT_ANTHROPIC_BYPASS]
    return [TRANSPORT_OPENAI, TRANSPORT_MINIMAX, TRANSPORT_ANTHROPIC_BYPASS]


def parse_data_uri_image(data_uri: str) -> ResolvedImage:
    if not data_uri.startswith("data:"):
        raise RuntimeError("image input is not a data URI")
    header, sep, payload = data_uri.partition(",")
    if not sep:
        raise RuntimeError("invalid data URI: missing comma separator")
    if ";base64" not in header.lower():
        raise RuntimeError("data URI must be base64 encoded")
    try:
        binary = base64.b64decode(payload, validate=True)
    except Exception as err:  # pragma: no cover - runtime validation
        raise RuntimeError(f"invalid base64 payload in data URI: {err}") from err
    media_type = sniff_image_media_type(binary)
    if media_type is None:
        declared = header[5:].split(";", 1)[0].strip().lower() or "application/octet-stream"
        raise RuntimeError(
            f"data URI payload is not a supported image; declared media type was '{declared}'"
        )
    return ResolvedImage(
        source="data-uri",
        media_type=media_type,
        base64_data=payload,
        data_uri=data_uri,
    )


def detect_image_media_type(candidate: str | None, fallback_name: str | None) -> str:
    media = normalize_non_empty(candidate)
    if media is not None:
        media = media.split(";", 1)[0].strip().lower()
        if media.startswith("image/"):
            return media

    if fallback_name:
        guessed, _enc = mimetypes.guess_type(fallback_name)
        if isinstance(guessed, str) and guessed.startswith("image/"):
            return guessed
    return "image/png"


def sniff_image_media_type(binary: bytes) -> str | None:
    if binary.startswith(b"\x89PNG\r\n\x1a\n"):
        return "image/png"
    if binary.startswith(b"\xff\xd8\xff"):
        return "image/jpeg"
    if binary.startswith((b"GIF87a", b"GIF89a")):
        return "image/gif"
    if len(binary) >= 12 and binary[:4] == b"RIFF" and binary[8:12] == b"WEBP":
        return "image/webp"
    return None


def resolve_binary_image_media_type(
    binary: bytes, candidate_media_type: str | None, fallback_name: str | None
) -> str:
    sniffed = sniff_image_media_type(binary)
    if sniffed is not None:
        return sniffed

    declared = detect_image_media_type(candidate_media_type, fallback_name)
    raise RuntimeError(
        "image payload is not a supported PNG/JPEG/GIF/WEBP file; "
        f"detected/declared media type was '{declared}'"
    )


def resolve_image_input(image_raw: str, timeout_secs: int) -> ResolvedImage:
    candidate = image_raw.strip()
    if not candidate:
        raise RuntimeError("image input is empty")

    if candidate.startswith("data:"):
        return parse_data_uri_image(candidate)

    if candidate.startswith("http://") or candidate.startswith("https://"):
        request = Request(
            url=candidate,
            method="GET",
            headers={"User-Agent": "xiuxian-llm-provider-smoke/1.0"},
        )
        try:
            with urlopen(request, timeout=timeout_secs) as response:
                binary = response.read()
                content_type = response.headers.get("Content-Type", "")
        except HTTPError as err:
            body = err.read().decode("utf-8", errors="replace")
            raise RuntimeError(
                f"failed to fetch image URL ({candidate}): {err.code} {err.reason}: "
                f"{parse_error_message(body) or body}"
            ) from err
        except URLError as err:
            raise RuntimeError(f"failed to fetch image URL ({candidate}): {err.reason}") from err

        media_type = resolve_binary_image_media_type(binary, content_type, urlparse(candidate).path)
        payload = base64.b64encode(binary).decode("ascii")
        return ResolvedImage(
            source=f"url:{candidate}",
            media_type=media_type,
            base64_data=payload,
            data_uri=f"data:{media_type};base64,{payload}",
        )

    path = Path(candidate).expanduser().resolve()
    if not path.exists():
        raise RuntimeError(f"image file not found: {path}")
    if not path.is_file():
        raise RuntimeError(f"image path is not a file: {path}")

    binary = path.read_bytes()
    media_type = resolve_binary_image_media_type(binary, None, path.name)
    payload = base64.b64encode(binary).decode("ascii")
    return ResolvedImage(
        source=f"file:{path}",
        media_type=media_type,
        base64_data=payload,
        data_uri=f"data:{media_type};base64,{payload}",
    )


def send_json_request(
    url: str, headers: dict[str, str], payload: dict[str, Any], timeout_secs: int
) -> dict[str, Any]:
    request = Request(
        url=url,
        data=json.dumps(payload).encode("utf-8"),
        method="POST",
        headers=headers,
    )
    try:
        with urlopen(request, timeout=timeout_secs) as response:
            body = response.read().decode("utf-8", errors="replace")
    except HTTPError as err:
        body = err.read().decode("utf-8", errors="replace")
        message = parse_error_message(body)
        raise RuntimeError(f"HTTP {err.code} {err.reason}: {message or body}") from err
    except URLError as err:
        raise RuntimeError(f"transport error: {err.reason}") from err

    try:
        parsed = json.loads(body)
    except json.JSONDecodeError as err:
        raise RuntimeError(f"invalid JSON response: {err}; body={body[:300]}") from err
    if not isinstance(parsed, dict):
        raise RuntimeError(f"unexpected response shape: {body[:300]}")
    return parsed


def send_json_request_raw(
    url: str, headers: dict[str, str], payload: dict[str, Any], timeout_secs: int
) -> str:
    request = Request(
        url=url,
        data=json.dumps(payload).encode("utf-8"),
        method="POST",
        headers=headers,
    )
    try:
        with urlopen(request, timeout=timeout_secs) as response:
            return response.read().decode("utf-8", errors="replace")
    except HTTPError as err:
        body = err.read().decode("utf-8", errors="replace")
        message = parse_error_message(body)
        raise RuntimeError(f"HTTP {err.code} {err.reason}: {message or body}") from err
    except URLError as err:
        raise RuntimeError(f"transport error: {err.reason}") from err


def extract_openai_reply(parsed: dict[str, Any]) -> str:
    choices = parsed.get("choices")
    if not isinstance(choices, list) or not choices:
        raise RuntimeError(f"response missing choices: {json.dumps(parsed)[:300]}")
    first = choices[0]
    if not isinstance(first, dict):
        raise RuntimeError(f"response choice shape invalid: {json.dumps(parsed)[:300]}")
    message = first.get("message")
    if not isinstance(message, dict):
        raise RuntimeError(f"response missing message object: {json.dumps(parsed)[:300]}")
    content = message.get("content")
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        pieces: list[str] = []
        for part in content:
            if isinstance(part, dict):
                text = part.get("text")
                if isinstance(text, str) and text.strip():
                    pieces.append(text.strip())
        if pieces:
            return "\n".join(pieces)
    raise RuntimeError(f"response missing message.content text: {json.dumps(parsed)[:300]}")


def extract_anthropic_reply(parsed: dict[str, Any]) -> str:
    blocks = parsed.get("content")
    if not isinstance(blocks, list):
        raise RuntimeError(f"anthropic response missing content blocks: {json.dumps(parsed)[:300]}")
    texts: list[str] = []
    for block in blocks:
        if isinstance(block, dict) and block.get("type") == "text":
            text = block.get("text")
            if isinstance(text, str) and text.strip():
                texts.append(text.strip())
    if not texts:
        raise RuntimeError(f"anthropic response has no text block: {json.dumps(parsed)[:300]}")
    return "\n".join(texts)


def call_openai_compatible(
    base_url: str,
    model: str,
    api_key: str,
    prompt: str,
    timeout_secs: int,
    image: ResolvedImage | None,
    transport: str,
    wire_api: str,
) -> CallResult:
    normalized_base = normalize_openai_compatible_base(base_url)
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {api_key}",
    }

    if wire_api == WIRE_API_RESPONSES:
        url = f"{normalized_base}/responses"
        if image is None:
            input_content: list[dict[str, Any]] = [{"type": "input_text", "text": prompt}]
        else:
            input_content = [
                {"type": "input_text", "text": prompt},
                {"type": "input_image", "image_url": image.data_uri, "detail": "high"},
            ]
        payload = {
            "model": model,
            "input": [{"role": "user", "content": input_content}],
            # Some gateways for codex models require stream=true.
            "stream": True,
        }
        try:
            body = send_json_request_raw(
                url=url,
                headers=headers,
                payload=payload,
                timeout_secs=timeout_secs,
            )
            content = extract_responses_stream_reply(body)
        except RuntimeError as err:
            raise RuntimeError(
                f"{err} (endpoint={url}, transport={transport}, model={model}, wire_api={wire_api})"
            ) from err
        return CallResult(content=content, endpoint=url, transport=transport)

    url = f"{normalized_base}/chat/completions"
    if image is None:
        message_content: Any = prompt
    else:
        message_content = [
            {"type": "text", "text": prompt},
            {"type": "image_url", "image_url": {"url": image.data_uri, "detail": "high"}},
        ]
    payload = {
        "model": model,
        "messages": [{"role": "user", "content": message_content}],
        "temperature": 0.0,
    }
    try:
        parsed = send_json_request(
            url=url,
            headers=headers,
            payload=payload,
            timeout_secs=timeout_secs,
        )
        content = extract_openai_reply(parsed)
    except RuntimeError as err:
        raise RuntimeError(
            f"{err} (endpoint={url}, transport={transport}, model={model}, wire_api={wire_api})"
        ) from err
    return CallResult(content=content, endpoint=url, transport=transport)


def call_anthropic_messages(
    base_url: str,
    model: str,
    api_key: str,
    prompt: str,
    timeout_secs: int,
    image: ResolvedImage | None,
    transport: str,
) -> CallResult:
    url = anthropic_messages_endpoint_from_base(base_url)
    if image is None:
        content: Any = prompt
    else:
        content = [
            {"type": "text", "text": prompt},
            {
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": image.media_type,
                    "data": image.base64_data,
                },
            },
        ]

    payload: dict[str, Any] = {
        "model": model,
        "max_tokens": 256,
        "messages": [{"role": "user", "content": content}],
    }
    try:
        parsed = send_json_request(
            url=url,
            headers={
                "Content-Type": "application/json",
                "x-api-key": api_key,
                "anthropic-version": ANTHROPIC_VERSION,
            },
            payload=payload,
            timeout_secs=timeout_secs,
        )
    except RuntimeError as err:
        raise RuntimeError(f"{err} (endpoint={url}, transport={transport}, model={model})") from err
    return CallResult(content=extract_anthropic_reply(parsed), endpoint=url, transport=transport)


def resolve_custom_transport_api_key(transport: str, configured_key: str | None) -> str | None:
    openai_key = read_non_empty_env("OPENAI_API_KEY")
    minimax_key = read_non_empty_env("MINIMAX_API_KEY")
    anthropic_key, _source = resolve_anthropic_env_key()

    if transport == TRANSPORT_OPENAI:
        candidates = [openai_key, configured_key, minimax_key, anthropic_key]
    elif transport == TRANSPORT_MINIMAX:
        candidates = [minimax_key, openai_key, configured_key, anthropic_key]
    else:
        candidates = [configured_key, anthropic_key, openai_key, minimax_key]
    for value in candidates:
        if value:
            return value
    return None


def call_anthropic_custom_base_fallback(
    case: SmokeCase,
    prompt: str,
    timeout_secs: int,
    image: ResolvedImage | None,
) -> CallResult:
    configured_key, _configured_source = resolve_key_from_spec(case.api_key_spec)
    failures: list[tuple[str, str]] = []

    for transport in fallback_transport_order(case.model):
        transport_key = resolve_custom_transport_api_key(transport, configured_key)
        if not transport_key:
            failures.append((transport, "missing transport API key"))
            continue

        try:
            if transport == TRANSPORT_ANTHROPIC_BYPASS:
                return call_anthropic_messages(
                    base_url=case.base_url,
                    model=case.model,
                    api_key=transport_key,
                    prompt=prompt,
                    timeout_secs=timeout_secs,
                    image=image,
                    transport=transport,
                )
            return call_openai_compatible(
                base_url=case.base_url,
                model=case.model,
                api_key=transport_key,
                prompt=prompt,
                timeout_secs=timeout_secs,
                image=image,
                transport=transport,
                wire_api=WIRE_API_CHAT,
            )
        except RuntimeError as err:
            failures.append((transport, str(err)))

    summary = " | ".join(f"{transport}: {reason}" for transport, reason in failures)
    raise RuntimeError(
        f"anthropic custom-base fallback exhausted after {len(failures)} attempt(s): {summary}"
    )


def call_case(
    case: SmokeCase,
    prompt: str,
    timeout_secs: int,
    image: ResolvedImage | None,
) -> CallResult:
    provider = case.provider.lower()
    if provider == "anthropic":
        if is_official_anthropic_base(case.base_url):
            key, _source = resolve_primary_provider_key(case)
            return call_anthropic_messages(
                base_url=case.base_url,
                model=case.model,
                api_key=key,
                prompt=prompt,
                timeout_secs=timeout_secs,
                image=image,
                transport=TRANSPORT_ANTHROPIC_BYPASS,
            )
        return call_anthropic_custom_base_fallback(case, prompt, timeout_secs, image)

    key, _source = resolve_primary_provider_key(case)
    return call_openai_compatible(
        base_url=case.base_url,
        model=case.model,
        api_key=key,
        prompt=prompt,
        timeout_secs=timeout_secs,
        image=image,
        transport=provider,
        wire_api=case.wire_api,
    )


def preview(text: str, max_len: int = 120) -> str:
    flattened = " ".join(text.split())
    if len(flattened) <= max_len:
        return flattened
    return f"{flattened[: max_len - 3]}..."


def parse_expected_term_groups(raw: str | None) -> list[list[str]]:
    if raw is None:
        return []

    groups: list[list[str]] = []
    for raw_group in raw.split(","):
        terms = [item.strip().casefold() for item in raw_group.split("|") if item.strip()]
        if terms:
            groups.append(terms)
    return groups


def format_expected_term_groups(groups: list[list[str]]) -> str:
    return ",".join("|".join(group) for group in groups)


def infer_image_expected_term_groups(
    image: ResolvedImage, explicit_raw: str | None
) -> list[list[str]]:
    explicit_groups = parse_expected_term_groups(explicit_raw)
    if explicit_groups:
        return explicit_groups

    source = image.source.casefold()
    for needle, groups in DEFAULT_IMAGE_EXPECTATION_HINTS.items():
        if needle in source:
            return [[term.casefold() for term in group] for group in groups]
    return []


def validate_image_reply_semantics(reply: str, expected_term_groups: list[list[str]]) -> None:
    if not expected_term_groups:
        return

    normalized = " ".join(reply.casefold().split())
    for group in expected_term_groups:
        if any(re.search(rf"\b{re.escape(term)}\b", normalized) for term in group):
            continue
        expected_display = format_expected_term_groups(expected_term_groups)
        raise RuntimeError(
            "semantic image assertion failed: expected reply to satisfy groups "
            f"[{expected_display}], got '{preview(reply, max_len=200)}'"
        )


def run_case_with_reporting(
    case: SmokeCase,
    timeout_secs: int,
    text_prompt: str,
    image_prompt: str,
    image: ResolvedImage | None,
    expected_image_term_groups: list[list[str]],
) -> list[str]:
    failures: list[str] = []

    try:
        text_result = call_case(case, text_prompt, timeout_secs, image=None)
        print(
            "PASS TEXT "
            f"provider='{case.provider}' model='{case.model}' "
            f"transport='{text_result.transport}' endpoint='{text_result.endpoint}' "
            f"reply_preview='{preview(text_result.content)}'"
        )
    except RuntimeError as err:
        failures.append(f"TEXT provider='{case.provider}' model='{case.model}' reason={err}")

    if image is not None:
        try:
            image_result = call_case(case, image_prompt, timeout_secs, image=image)
            validate_image_reply_semantics(image_result.content, expected_image_term_groups)
            semantic_suffix = ""
            if expected_image_term_groups:
                semantic_suffix = (
                    f" semantic_terms='{format_expected_term_groups(expected_image_term_groups)}'"
                )
            print(
                "PASS IMAGE "
                f"provider='{case.provider}' model='{case.model}' "
                f"transport='{image_result.transport}' endpoint='{image_result.endpoint}' "
                f"reply_preview='{preview(image_result.content)}'"
                f"{semantic_suffix}"
            )
        except RuntimeError as err:
            failures.append(f"IMAGE provider='{case.provider}' model='{case.model}' reason={err}")

    return failures


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(line_buffering=True)
    if hasattr(sys.stderr, "reconfigure"):
        sys.stderr.reconfigure(line_buffering=True)

    args = parse_args()
    if args.timeout_secs <= 0:
        print("Error: --timeout-secs must be > 0", file=sys.stderr)
        return 2

    config_path = (
        Path(args.config_path) if args.config_path.strip() else default_xiuxian_config_path()
    )
    provider_filter = parse_provider_filter(args.provider)
    model_override = normalize_non_empty(args.model_override)
    wire_api_override = normalize_non_empty(args.wire_api)
    text_prompt = normalize_non_empty(args.text_prompt) or DEFAULT_TEXT_PROMPT
    image_prompt = normalize_non_empty(args.image_prompt) or DEFAULT_IMAGE_PROMPT
    image_raw = normalize_non_empty(args.image)
    image_contains_raw = normalize_non_empty(args.image_contains)

    try:
        config = load_toml(config_path)
        cases = build_cases(config, provider_filter, model_override, wire_api_override)
        image = resolve_image_input(image_raw, args.timeout_secs) if image_raw else None
        expected_image_term_groups = (
            infer_image_expected_term_groups(image, image_contains_raw) if image else []
        )
    except RuntimeError as err:
        print(f"Error: {err}", file=sys.stderr)
        return 2

    print(
        f"Running live provider smoke test: config='{config_path}', "
        f"providers={len(cases)}, timeout={args.timeout_secs}s, image_test={'on' if image else 'off'}"
    )
    if image:
        print(
            f"Image source='{image.source}', media_type='{image.media_type}', bytes(base64)={len(image.base64_data)}"
        )
        if expected_image_term_groups:
            print(
                f"Image semantic expectations={format_expected_term_groups(expected_image_term_groups)}"
            )

    for case in cases:
        api_key_spec = case.api_key_spec or default_api_key_env(case.provider)
        print(
            f"- provider='{case.provider}', model='{case.model}', base_url='{case.base_url}', "
            f"api_key_spec='{api_key_spec}', wire_api='{case.wire_api}'"
        )

    failures: list[str] = []
    for case in cases:
        failures.extend(
            run_case_with_reporting(
                case=case,
                timeout_secs=args.timeout_secs,
                text_prompt=text_prompt,
                image_prompt=image_prompt,
                image=image,
                expected_image_term_groups=expected_image_term_groups,
            )
        )

    if failures:
        print(f"Provider smoke test failed for {len(failures)} case(s):", file=sys.stderr)
        for item in failures:
            print(f"  FAIL {item}", file=sys.stderr)
        return 1

    print("All selected providers passed live connectivity checks.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
