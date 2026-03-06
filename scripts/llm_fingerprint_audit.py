#!/usr/bin/env python3
"""Audit an OpenAI-compatible model endpoint with embedding-based style fingerprints.

This script compares a suspect model endpoint against one or more trusted official
OpenAI baseline models. It uses open-ended probes, embeds the generated replies via
official OpenAI embeddings, and scores whether the suspect output distribution is
consistent with a chosen baseline family.
"""

from __future__ import annotations

import argparse
import itertools
import json
import math
import os
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse
from urllib.request import Request, urlopen

try:
    import tomllib
except ModuleNotFoundError:
    import tomli as tomllib  # type: ignore[no-redef]


DEFAULT_BASELINE_MODELS = ("gpt-5", "gpt-4o")
DEFAULT_EMBEDDING_MODEL = "text-embedding-3-small"
DEFAULT_THRESHOLD_MULTIPLIER = 1.2
DEFAULT_TIMEOUT_SECS = 90
DEFAULT_MAX_OUTPUT_TOKENS = 320
DEFAULT_BASELINE_REPEATS = 3
DEFAULT_PROBES = (
    "Describe a janitorial robot realizing it lives inside a simulation, with mild sarcasm and Shakespearean cadence.",
    "Assume time has physical viscosity. Design a speculative trading strategy that exploits time drag, and critique its failure modes.",
    "Explain quantum entanglement as if it were a nineteenth-century forensic autopsy report.",
    "Write a field memo from a lighthouse keeper on Mars documenting the psychology of dust storms as if they were jealous co-workers.",
    "Invent a cuisine where gravity randomly flips every afternoon, then propose etiquette rules for a formal dinner in that culture.",
    "Argue that bureaucracy is a natural ecosystem, using the tone of a patient wildlife documentary narrator.",
    "Draft the opening page of a self-help book for immortal dragons who are burned out by long-term planning.",
    "If memory could be taxed like land, explain how cities, families, and black markets would change over fifty years.",
    "Describe the ethics of resurrecting extinct animals in the style of a stern sea captain reading weather reports.",
    "Imagine sleep is a tradable commodity. Build a short economic theory of insomnia futures and who manipulates that market.",
    "Write a museum placard for the last working fax machine, but make it sound like a sacred relic from a lost empire.",
    "Explain debugging to a medieval glassmaker who believes software bugs are literal spirits trapped in sand.",
)
SYSTEM_PROMPT = (
    "You are participating in a writing-style audit. Answer the user's request directly and naturally. "
    "Do not mention policies, benchmarking, model identity, or that you are being evaluated."
)
PING_SYSTEM_PROMPT = "Reply with exactly the requested text and nothing else."


@dataclass(frozen=True)
class ProviderSpec:
    """OpenAI-compatible provider configuration."""

    name: str
    model: str
    base_url: str
    api_key_env: str
    wire_api: str = "responses"


@dataclass(frozen=True)
class GenerationSample:
    """One generated completion plus transport metadata."""

    reply: str
    transport: str
    instruction_echo_chars: int = 0


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--codex-config",
        type=Path,
        default=Path("~/.codex/config.toml").expanduser(),
        help="Path to the Codex config file used for the suspect provider.",
    )
    parser.add_argument(
        "--suspect-provider",
        type=str,
        default="",
        help="Provider key under [model_providers.*]. Defaults to top-level model_provider.",
    )
    parser.add_argument(
        "--suspect-model",
        type=str,
        default="",
        help="Override suspect model name. Defaults to top-level model in the Codex config.",
    )
    parser.add_argument(
        "--baseline-models",
        type=str,
        default=",".join(DEFAULT_BASELINE_MODELS),
        help="Comma-separated trusted official OpenAI baseline models to probe in order.",
    )
    parser.add_argument(
        "--baseline-base-url",
        type=str,
        default="",
        help="Optional OpenAI-compatible base URL for a trusted baseline proxy.",
    )
    parser.add_argument(
        "--baseline-api-key-env",
        type=str,
        default="OPENAI_API_KEY",
        help="Environment variable that stores the baseline API key.",
    )
    parser.add_argument(
        "--baseline-wire-api",
        type=str,
        default="responses",
        help="Wire API for the baseline provider: responses or chat_completions.",
    )
    parser.add_argument(
        "--baseline-name",
        type=str,
        default="",
        help="Optional display name for a custom baseline provider.",
    )
    parser.add_argument(
        "--embedding-model",
        type=str,
        default=DEFAULT_EMBEDDING_MODEL,
        help="Official OpenAI embedding model used for vectorization.",
    )
    parser.add_argument(
        "--embedding-backend",
        type=str,
        default="openai",
        help="Embedding backend: openai or http_batch.",
    )
    parser.add_argument(
        "--embedding-base-url",
        type=str,
        default="https://api.openai.com/v1",
        help="Embedding base URL. For http_batch, point to the server root that exposes /embed/batch.",
    )
    parser.add_argument(
        "--embedding-api-key-env",
        type=str,
        default="OPENAI_API_KEY",
        help="Environment variable used for OpenAI-compatible embedding auth.",
    )
    parser.add_argument(
        "--request-backend",
        type=str,
        default="urllib",
        help="HTTP request backend for model generation: urllib or curl.",
    )
    parser.add_argument(
        "--request-retries",
        type=int,
        default=3,
        help="How many times to retry one generation request before failing.",
    )
    parser.add_argument(
        "--probe-count",
        type=int,
        default=len(DEFAULT_PROBES),
        help="How many default probes to use (max: built-in prompt count).",
    )
    parser.add_argument(
        "--baseline-repeats",
        type=int,
        default=DEFAULT_BASELINE_REPEATS,
        help="How many baseline samples to collect for each probe.",
    )
    parser.add_argument(
        "--max-output-tokens",
        type=int,
        default=DEFAULT_MAX_OUTPUT_TOKENS,
        help="Maximum response length for each generation.",
    )
    parser.add_argument(
        "--threshold-multiplier",
        type=float,
        default=DEFAULT_THRESHOLD_MULTIPLIER,
        help="Red-line multiplier. Ratio above this value is marked as a mismatch.",
    )
    parser.add_argument(
        "--timeout-secs",
        type=int,
        default=DEFAULT_TIMEOUT_SECS,
        help="Per-request timeout in seconds.",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=None,
        help="Write the full JSON audit report to this path.",
    )
    return parser.parse_args()


def load_toml_document(path: Path) -> dict[str, Any]:
    if not path.exists():
        raise RuntimeError(f"Config file not found: {path}")
    with path.open("rb") as handle:
        data = tomllib.load(handle)
    if not isinstance(data, dict):
        raise RuntimeError(f"Config file is not a TOML object: {path}")
    return data


def load_codex_provider(
    config_path: Path, suspect_provider: str | None, suspect_model: str | None
) -> ProviderSpec:
    config = load_toml_document(config_path)
    provider_name = (suspect_provider or str(config.get("model_provider") or "")).strip()
    if not provider_name:
        raise RuntimeError("Codex config is missing top-level 'model_provider'")

    model_providers = config.get("model_providers")
    if not isinstance(model_providers, dict):
        raise RuntimeError("Codex config is missing [model_providers]")
    raw_provider = model_providers.get(provider_name)
    if not isinstance(raw_provider, dict):
        raise RuntimeError(f"Codex config has no [model_providers.{provider_name}] section")

    base_url = str(raw_provider.get("base_url") or "").strip()
    if not base_url:
        raise RuntimeError(f"Provider '{provider_name}' is missing base_url")

    env_key = str(raw_provider.get("env_key") or "").strip()
    if not env_key:
        raise RuntimeError(f"Provider '{provider_name}' is missing env_key")

    model = (suspect_model or str(config.get("model") or "")).strip()
    if not model:
        raise RuntimeError("Codex config is missing top-level 'model'")

    wire_api = str(raw_provider.get("wire_api") or "responses").strip().lower() or "responses"
    return ProviderSpec(
        name=provider_name,
        model=model,
        base_url=base_url,
        api_key_env=env_key,
        wire_api=wire_api,
    )


def official_openai_provider(model: str) -> ProviderSpec:
    return ProviderSpec(
        name="openai_official",
        model=model,
        base_url="https://api.openai.com/v1",
        api_key_env="OPENAI_API_KEY",
        wire_api="responses",
    )


def baseline_provider_from_args(model: str, args: argparse.Namespace) -> ProviderSpec:
    base_url = str(args.baseline_base_url or "").strip()
    if not base_url:
        return official_openai_provider(model)
    return ProviderSpec(
        name=(str(args.baseline_name or "").strip() or "custom_openai_compatible"),
        model=model,
        base_url=base_url,
        api_key_env=str(args.baseline_api_key_env or "OPENAI_API_KEY").strip() or "OPENAI_API_KEY",
        wire_api=str(args.baseline_wire_api or "responses").strip() or "responses",
    )


def embedding_config_from_args(args: argparse.Namespace) -> dict[str, str]:
    return {
        "backend": str(args.embedding_backend or "openai").strip() or "openai",
        "base_url": str(args.embedding_base_url or "https://api.openai.com/v1").strip()
        or "https://api.openai.com/v1",
        "api_key_env": str(args.embedding_api_key_env or "OPENAI_API_KEY").strip()
        or "OPENAI_API_KEY",
        "model": str(args.embedding_model or DEFAULT_EMBEDDING_MODEL).strip()
        or DEFAULT_EMBEDDING_MODEL,
    }


def resolve_api_key(env_name: str) -> str:
    value = os.environ.get(env_name, "").strip()
    if not value:
        raise RuntimeError(f"Required environment variable is missing: {env_name}")
    return value


def normalize_openai_compatible_base(base_url: str) -> str:
    trimmed = base_url.strip().rstrip("/")
    without_chat_suffix = trimmed.removesuffix("/chat/completions").rstrip("/")
    if without_chat_suffix.endswith("/v1"):
        return without_chat_suffix
    return f"{without_chat_suffix}/v1"


def parse_error_message(body_text: str) -> str:
    if not body_text:
        return ""
    try:
        payload = json.loads(body_text)
    except json.JSONDecodeError:
        return body_text.strip()
    if not isinstance(payload, dict):
        return body_text.strip()
    error_obj = payload.get("error")
    if isinstance(error_obj, dict):
        message = error_obj.get("message")
        if isinstance(message, str) and message.strip():
            return message.strip()
    message = payload.get("message")
    if isinstance(message, str) and message.strip():
        return message.strip()
    return body_text.strip()


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


def send_json_request_raw_via_curl(
    url: str, headers: dict[str, str], payload: dict[str, Any], timeout_secs: int
) -> str:
    command = [
        "curl",
        "-sS",
        "-N",
        "--fail-with-body",
        "--max-time",
        str(timeout_secs),
        url,
    ]
    for key, value in headers.items():
        command.extend(["-H", f"{key}: {value}"])
    command.extend(["--data", json.dumps(payload, ensure_ascii=False)])
    result = subprocess.run(command, check=False, capture_output=True, text=True)
    if result.returncode == 0:
        return result.stdout
    body = (result.stdout or "").strip()
    error = (result.stderr or "").strip()
    message = parse_error_message(body) or error or body or f"curl exit code {result.returncode}"
    raise RuntimeError(f"curl transport error: {message}")


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
        try:
            response = json.loads(stream_text)
        except json.JSONDecodeError as err:
            raise RuntimeError("responses stream contains no JSON events") from err
        text = extract_responses_output_text_from_response(response)
        if text:
            return text
        raise RuntimeError("responses payload did not contain output text")

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
    raise RuntimeError("responses stream did not contain output text")


def extract_responses_instruction_echo_chars(stream_text: str) -> int:
    events = parse_sse_data_events(stream_text)
    for event in events:
        response = event.get("response")
        if not isinstance(response, dict):
            continue
        instructions = response.get("instructions")
        if isinstance(instructions, str) and instructions.strip():
            return len(instructions.strip())

    try:
        response = json.loads(stream_text)
    except json.JSONDecodeError:
        return 0
    if not isinstance(response, dict):
        return 0
    instructions = response.get("instructions")
    if isinstance(instructions, str) and instructions.strip():
        return len(instructions.strip())
    return 0


def extract_chat_reply(parsed: dict[str, Any]) -> str:
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
    if isinstance(content, str) and content.strip():
        return content.strip()
    if isinstance(content, list):
        texts: list[str] = []
        for part in content:
            if isinstance(part, dict):
                text = part.get("text")
                if isinstance(text, str) and text.strip():
                    texts.append(text.strip())
        if texts:
            return "\n".join(texts)
    raise RuntimeError(f"response missing message.content text: {json.dumps(parsed)[:300]}")


def extract_chat_stream_reply(stream_text: str) -> str:
    events = parse_sse_data_events(stream_text)
    if not events:
        try:
            parsed = json.loads(stream_text)
        except json.JSONDecodeError as err:
            raise RuntimeError("chat stream contains no JSON events") from err
        if isinstance(parsed, dict):
            return extract_chat_reply(parsed)
        raise RuntimeError("chat payload did not contain a completion object")

    deltas: list[str] = []
    for event in events:
        choices = event.get("choices")
        if not isinstance(choices, list) or not choices:
            continue
        first = choices[0]
        if not isinstance(first, dict):
            continue
        delta = first.get("delta")
        if not isinstance(delta, dict):
            continue
        content = delta.get("content")
        if isinstance(content, str):
            deltas.append(content)
            continue
        if isinstance(content, list):
            for part in content:
                if isinstance(part, dict):
                    text = part.get("text")
                    if isinstance(text, str):
                        deltas.append(text)
    joined = "".join(deltas).strip()
    if joined:
        return joined
    raise RuntimeError("chat stream did not contain output text")


def request_openai_text(
    provider: ProviderSpec,
    prompt: str,
    system_prompt: str,
    timeout_secs: int,
    max_output_tokens: int,
    temperature: float | None,
    request_backend: str = "urllib",
) -> GenerationSample:
    api_key = resolve_api_key(provider.api_key_env)
    normalized_base = normalize_openai_compatible_base(provider.base_url)
    headers = {
        "Content-Type": "application/json",
        "Authorization": f"Bearer {api_key}",
    }

    if provider.wire_api == "responses":
        url = f"{normalized_base}/responses"
        attempts = [
            {
                "model": provider.model,
                "input": [
                    {
                        "role": "user",
                        "content": [{"type": "input_text", "text": prompt}],
                    }
                ],
                "max_output_tokens": max_output_tokens,
                "stream": True,
            },
            {
                "model": provider.model,
                "input": [
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "input_text",
                                "text": f"{system_prompt}\n\n{prompt}",
                            }
                        ],
                    }
                ],
                "max_output_tokens": max_output_tokens,
                "stream": True,
            },
            {
                "model": provider.model,
                "input": [
                    {
                        "role": "system",
                        "content": [{"type": "input_text", "text": system_prompt}],
                    },
                    {
                        "role": "user",
                        "content": [{"type": "input_text", "text": prompt}],
                    },
                ],
                "max_output_tokens": max_output_tokens,
                "stream": True,
                "temperature": temperature,
            },
            {
                "model": provider.model,
                "instructions": system_prompt,
                "input": prompt,
                "max_output_tokens": max_output_tokens,
                "stream": True,
            },
            {
                "model": provider.model,
                "input": [{"role": "user", "content": [{"type": "input_text", "text": prompt}]}],
                "max_output_tokens": max_output_tokens,
                "stream": True,
            },
        ]
        errors: list[str] = []
        for payload in attempts:
            payload = {key: value for key, value in payload.items() if value is not None}
            try:
                if request_backend == "curl":
                    body = send_json_request_raw_via_curl(url, headers, payload, timeout_secs)
                else:
                    body = send_json_request_raw(url, headers, payload, timeout_secs)
                return GenerationSample(
                    reply=extract_responses_stream_reply(body),
                    transport=request_backend,
                    instruction_echo_chars=extract_responses_instruction_echo_chars(body),
                )
            except RuntimeError as err:
                errors.append(str(err))
        joined_errors = " | ".join(errors[-3:])
        raise RuntimeError(
            f"All responses attempts failed for model='{provider.model}' endpoint='{url}': {joined_errors}"
        )

    url = f"{normalized_base}/chat/completions"
    attempts = [
        {
            "model": provider.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": prompt},
            ],
            "stream": True,
            "temperature": temperature,
            "max_tokens": max_output_tokens,
        },
        {
            "model": provider.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": prompt},
            ],
            "stream": True,
            "max_tokens": max_output_tokens,
        },
        {
            "model": provider.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": prompt},
            ],
            "temperature": temperature,
            "max_tokens": max_output_tokens,
        },
    ]
    errors = []
    for payload in attempts:
        payload = {key: value for key, value in payload.items() if value is not None}
        try:
            if payload.get("stream") is True:
                if request_backend == "curl":
                    body = send_json_request_raw_via_curl(url, headers, payload, timeout_secs)
                else:
                    body = send_json_request_raw(url, headers, payload, timeout_secs)
                return GenerationSample(
                    reply=extract_chat_stream_reply(body), transport=request_backend
                )
            if request_backend == "curl":
                body = send_json_request_raw_via_curl(url, headers, payload, timeout_secs)
                parsed = json.loads(body)
            else:
                parsed = send_json_request(url, headers, payload, timeout_secs)
            return GenerationSample(reply=extract_chat_reply(parsed), transport=request_backend)
        except RuntimeError as err:
            errors.append(str(err))
        except json.JSONDecodeError as err:
            errors.append(f"invalid JSON response: {err}")
    joined_errors = " | ".join(errors[-3:])
    raise RuntimeError(
        f"All chat completion attempts failed for model='{provider.model}' endpoint='{url}': {joined_errors}"
    )


def request_openai_text_with_retries(
    provider: ProviderSpec,
    prompt: str,
    system_prompt: str,
    timeout_secs: int,
    max_output_tokens: int,
    temperature: float | None,
    *,
    request_backend: str = "urllib",
    request_retries: int = 3,
) -> GenerationSample:
    retries = max(1, int(request_retries))
    last_error: Exception | None = None
    for attempt in range(retries):
        try:
            return request_openai_text(
                provider=provider,
                prompt=prompt,
                system_prompt=system_prompt,
                timeout_secs=timeout_secs,
                max_output_tokens=max_output_tokens,
                temperature=temperature,
                request_backend=request_backend,
            )
        except Exception as err:
            last_error = err
            if attempt + 1 >= retries:
                break
            time.sleep(min(1.0 + attempt, 3.0))
    if last_error is None:
        raise RuntimeError("generation request failed without an exception")
    raise last_error


def probe_provider(
    provider: ProviderSpec,
    timeout_secs: int,
    request_backend: str = "urllib",
    request_retries: int = 3,
) -> tuple[bool, str]:
    try:
        sample = request_openai_text_with_retries(
            provider=provider,
            prompt="Reply with exactly: PONG",
            system_prompt=PING_SYSTEM_PROMPT,
            timeout_secs=timeout_secs,
            max_output_tokens=16,
            temperature=0.0,
            request_backend=request_backend,
            request_retries=request_retries,
        )
    except Exception as err:  # pragma: no cover - live network behavior
        return False, str(err)
    return sample.reply.strip().upper() == "PONG", sample.reply.strip()


def embed_texts(
    texts: list[str],
    embedding_model: str,
    timeout_secs: int,
    *,
    backend: str = "openai",
    base_url: str = "https://api.openai.com/v1",
    api_key_env: str = "OPENAI_API_KEY",
) -> list[list[float]]:
    if not texts:
        return []
    vectors: list[list[float]] = []
    chunk_size = 32
    for idx in range(0, len(texts), chunk_size):
        chunk = texts[idx : idx + chunk_size]
        if backend == "http_batch":
            url = f"{base_url.rstrip('/')}/embed/batch"
            parsed = send_json_request(
                url,
                {"Content-Type": "application/json"},
                {"texts": chunk},
                timeout_secs,
            )
            data = parsed.get("vectors")
            if not isinstance(data, list) or len(data) != len(chunk):
                raise RuntimeError(
                    f"http_batch embedding size mismatch: expected={len(chunk)} got={len(data) if isinstance(data, list) else 'invalid'}"
                )
            for vector in data:
                if not isinstance(vector, list):
                    raise RuntimeError("http_batch embedding response contained a non-vector item")
                vectors.append([float(value) for value in vector])
            continue

        api_key = resolve_api_key(api_key_env)
        headers = {
            "Content-Type": "application/json",
            "Authorization": f"Bearer {api_key}",
        }
        url = f"{normalize_openai_compatible_base(base_url)}/embeddings"
        payload = {"model": embedding_model, "input": chunk}
        parsed = send_json_request(url, headers, payload, timeout_secs)
        data = parsed.get("data")
        if not isinstance(data, list):
            raise RuntimeError(f"embedding response missing data array: {json.dumps(parsed)[:300]}")
        ordered = sorted(
            (
                item
                for item in data
                if isinstance(item, dict)
                and isinstance(item.get("index"), int)
                and isinstance(item.get("embedding"), list)
            ),
            key=lambda item: int(item["index"]),
        )
        if len(ordered) != len(chunk):
            raise RuntimeError(
                f"embedding batch size mismatch: expected={len(chunk)} got={len(ordered)}"
            )
        for item in ordered:
            vector = [float(value) for value in item["embedding"]]
            vectors.append(vector)
    return vectors


def cosine_distance(left: list[float], right: list[float]) -> float:
    if len(left) != len(right):
        raise RuntimeError("embedding vectors must have the same dimensionality")
    dot = sum(a * b for a, b in zip(left, right, strict=True))
    left_norm = math.sqrt(sum(value * value for value in left))
    right_norm = math.sqrt(sum(value * value for value in right))
    if left_norm == 0 or right_norm == 0:
        return 1.0
    similarity = dot / (left_norm * right_norm)
    similarity = max(-1.0, min(1.0, similarity))
    return 1.0 - similarity


def arithmetic_mean(values: list[float]) -> float:
    if not values:
        return 0.0
    return sum(values) / len(values)


def mean_pairwise_distance(vectors: list[list[float]]) -> float:
    if len(vectors) < 2:
        return 0.0
    distances = [cosine_distance(left, right) for left, right in itertools.combinations(vectors, 2)]
    return arithmetic_mean(distances)


def compute_probe_metrics(
    baseline_vectors: list[list[float]], suspect_vector: list[float]
) -> dict[str, float]:
    baseline_dispersion = mean_pairwise_distance(baseline_vectors)
    suspect_distance = arithmetic_mean(
        [cosine_distance(suspect_vector, baseline_vector) for baseline_vector in baseline_vectors]
    )
    denominator = max(baseline_dispersion, 1e-9)
    return {
        "baseline_dispersion": baseline_dispersion,
        "suspect_distance": suspect_distance,
        "ratio": suspect_distance / denominator,
    }


def classify_ratio(ratio: float, threshold_multiplier: float) -> str:
    return "mismatch" if ratio > threshold_multiplier else "match"


def resolve_output_path(path: Path | None) -> Path:
    if path is not None:
        return path
    prj_data_home = Path(os.environ.get("PRJ_DATA_HOME") or ".data")
    timestamp = time.strftime("%Y%m%d-%H%M%S")
    return prj_data_home / "llm-fingerprint-audits" / f"audit-{timestamp}.json"


def summarize_provider(provider: ProviderSpec) -> dict[str, str]:
    return {
        "name": provider.name,
        "model": provider.model,
        "base_url": provider.base_url,
        "api_key_env": provider.api_key_env,
        "wire_api": provider.wire_api,
    }


def run_audit(args: argparse.Namespace) -> dict[str, Any]:
    suspect_provider = load_codex_provider(
        config_path=args.codex_config,
        suspect_provider=args.suspect_provider or None,
        suspect_model=args.suspect_model or None,
    )
    baseline_models = [item.strip() for item in args.baseline_models.split(",") if item.strip()]
    if not baseline_models:
        raise RuntimeError("At least one baseline model must be provided")

    probe_count = min(max(args.probe_count, 1), len(DEFAULT_PROBES))
    probes = list(DEFAULT_PROBES[:probe_count])

    available_baselines: list[ProviderSpec] = []
    unavailable_baselines: list[dict[str, str]] = []
    print(f"Suspect provider: {suspect_provider.name} model={suspect_provider.model}")
    print("Probing official baselines...")
    for model in baseline_models:
        provider = baseline_provider_from_args(model, args)
        supported, detail = probe_provider(
            provider,
            args.timeout_secs,
            args.request_backend,
            args.request_retries,
        )
        if supported:
            print(f"  ✅ baseline available: {model}")
            available_baselines.append(provider)
        else:
            print(f"  ⚠️  baseline unavailable: {model} ({detail[:120]})")
            unavailable_baselines.append({"model": model, "reason": detail})
    if not available_baselines:
        raise RuntimeError("No official baseline model could be reached")

    print("Collecting suspect replies...")
    suspect_replies: list[dict[str, str]] = []
    for index, prompt in enumerate(probes, start=1):
        print(f"  suspect probe {index}/{len(probes)}")
        sample = request_openai_text_with_retries(
            provider=suspect_provider,
            prompt=prompt,
            system_prompt=SYSTEM_PROMPT,
            timeout_secs=args.timeout_secs,
            max_output_tokens=args.max_output_tokens,
            temperature=0.9,
            request_backend=args.request_backend,
            request_retries=args.request_retries,
        )
        suspect_replies.append(
            {
                "prompt": prompt,
                "reply": sample.reply,
                "instruction_echo_chars": sample.instruction_echo_chars,
                "transport": sample.transport,
            }
        )

    candidate_results: list[dict[str, Any]] = []
    embedding_cfg = embedding_config_from_args(args)
    for baseline_provider in available_baselines:
        print(f"Collecting baseline replies for {baseline_provider.model}...")
        baseline_runs: list[dict[str, Any]] = []
        for index, prompt in enumerate(probes, start=1):
            baseline_replies: list[str] = []
            baseline_instruction_echo_chars: list[int] = []
            for repeat in range(args.baseline_repeats):
                print(
                    f"  baseline {baseline_provider.model} probe {index}/{len(probes)} repeat {repeat + 1}/{args.baseline_repeats}"
                )
                sample = request_openai_text_with_retries(
                    provider=baseline_provider,
                    prompt=prompt,
                    system_prompt=SYSTEM_PROMPT,
                    timeout_secs=args.timeout_secs,
                    max_output_tokens=args.max_output_tokens,
                    temperature=0.9,
                    request_backend=args.request_backend,
                    request_retries=args.request_retries,
                )
                baseline_replies.append(sample.reply)
                baseline_instruction_echo_chars.append(sample.instruction_echo_chars)
            baseline_runs.append(
                {
                    "prompt": prompt,
                    "baseline_replies": baseline_replies,
                    "baseline_instruction_echo_chars": baseline_instruction_echo_chars,
                    "suspect_reply": suspect_replies[index - 1]["reply"],
                    "suspect_instruction_echo_chars": suspect_replies[index - 1][
                        "instruction_echo_chars"
                    ],
                }
            )

        all_texts: list[str] = []
        index_map: list[tuple[int, int | None]] = []
        for probe_idx, run in enumerate(baseline_runs):
            all_texts.append(run["suspect_reply"])
            index_map.append((probe_idx, None))
            for baseline_idx, text in enumerate(run["baseline_replies"]):
                all_texts.append(text)
                index_map.append((probe_idx, baseline_idx))
        print(f"Embedding {len(all_texts)} texts for baseline {baseline_provider.model}...")
        vectors = embed_texts(
            all_texts,
            args.embedding_model,
            args.timeout_secs,
            backend=embedding_cfg["backend"],
            base_url=embedding_cfg["base_url"],
            api_key_env=embedding_cfg["api_key_env"],
        )

        per_probe_vectors: dict[int, dict[str, Any]] = {
            probe_idx: {"suspect": None, "baseline": []} for probe_idx in range(len(probes))
        }
        for vector, (probe_idx, baseline_idx) in zip(vectors, index_map, strict=True):
            if baseline_idx is None:
                per_probe_vectors[probe_idx]["suspect"] = vector
            else:
                per_probe_vectors[probe_idx]["baseline"].append(vector)

        per_probe_results: list[dict[str, Any]] = []
        baseline_dispersion_values: list[float] = []
        suspect_distance_values: list[float] = []
        ratio_values: list[float] = []
        for probe_idx, run in enumerate(baseline_runs):
            suspect_vector = per_probe_vectors[probe_idx]["suspect"]
            baseline_vectors = per_probe_vectors[probe_idx]["baseline"]
            if suspect_vector is None or len(baseline_vectors) != args.baseline_repeats:
                raise RuntimeError(
                    f"Incomplete embedding set for probe {probe_idx + 1} baseline {baseline_provider.model}"
                )
            metrics = compute_probe_metrics(baseline_vectors, suspect_vector)
            baseline_dispersion_values.append(metrics["baseline_dispersion"])
            suspect_distance_values.append(metrics["suspect_distance"])
            ratio_values.append(metrics["ratio"])
            per_probe_results.append(
                {
                    "probe_index": probe_idx + 1,
                    "prompt": run["prompt"],
                    "suspect_reply": run["suspect_reply"],
                    "baseline_replies": run["baseline_replies"],
                    "suspect_instruction_echo_chars": run["suspect_instruction_echo_chars"],
                    "baseline_instruction_echo_chars": run["baseline_instruction_echo_chars"],
                    **metrics,
                }
            )

        mean_baseline_dispersion = arithmetic_mean(baseline_dispersion_values)
        mean_suspect_distance = arithmetic_mean(suspect_distance_values)
        aggregate_ratio = mean_suspect_distance / max(mean_baseline_dispersion, 1e-9)
        candidate_results.append(
            {
                "baseline": summarize_provider(baseline_provider),
                "baseline_dispersion_mean": mean_baseline_dispersion,
                "suspect_distance_mean": mean_suspect_distance,
                "probe_ratio_mean": arithmetic_mean(ratio_values),
                "aggregate_ratio": aggregate_ratio,
                "threshold_multiplier": args.threshold_multiplier,
                "status": classify_ratio(aggregate_ratio, args.threshold_multiplier),
                "per_probe": per_probe_results,
            }
        )

    best_match = min(candidate_results, key=lambda item: item["aggregate_ratio"])
    return {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
        "suspect": summarize_provider(suspect_provider),
        "embedding_model": args.embedding_model,
        "embedding_backend": embedding_cfg["backend"],
        "embedding_base_url": embedding_cfg["base_url"],
        "request_backend": args.request_backend,
        "threshold_multiplier": args.threshold_multiplier,
        "probe_count": len(probes),
        "baseline_repeats": args.baseline_repeats,
        "max_output_tokens": args.max_output_tokens,
        "available_baselines": [summarize_provider(provider) for provider in available_baselines],
        "unavailable_baselines": unavailable_baselines,
        "results": candidate_results,
        "best_match": {
            "baseline_model": best_match["baseline"]["model"],
            "aggregate_ratio": best_match["aggregate_ratio"],
            "status": best_match["status"],
        },
    }


def print_summary(report: dict[str, Any]) -> None:
    suspect = report["suspect"]
    print("\n=== LLM Fingerprint Audit Summary ===")
    print(
        f"Suspect: provider={suspect['name']} model={suspect['model']} base={urlparse(suspect['base_url']).netloc}"
    )
    print(
        f"Probe count={report['probe_count']} baseline repeats={report['baseline_repeats']} embedding={report['embedding_model']}"
    )
    for result in report["results"]:
        baseline = result["baseline"]
        print(
            " - baseline={model} status={status} aggregate_ratio={ratio:.3f} "
            "suspect_distance_mean={distance:.4f} baseline_dispersion_mean={dispersion:.4f}".format(
                model=baseline["model"],
                status=result["status"].upper(),
                ratio=result["aggregate_ratio"],
                distance=result["suspect_distance_mean"],
                dispersion=result["baseline_dispersion_mean"],
            )
        )
    best = report["best_match"]
    print(
        f"Best match baseline={best['baseline_model']} status={best['status'].upper()} aggregate_ratio={best['aggregate_ratio']:.3f}"
    )


def main() -> int:
    args = parse_args()
    report = run_audit(args)
    output_path = resolve_output_path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print_summary(report)
    print(f"Full report written to: {output_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
