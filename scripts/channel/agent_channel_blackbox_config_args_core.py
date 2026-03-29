#!/usr/bin/env python3
"""Core CLI argument group for agent channel blackbox config."""

from __future__ import annotations

import os
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import argparse


def add_core_args(parser: argparse.ArgumentParser, *, webhook_url_default: str) -> None:
    """Register core probe runtime/input arguments."""
    parser.add_argument("--prompt", required=True, help="Prompt to inject.")
    parser.add_argument(
        "--max-wait",
        type=int,
        default=None,
        help="Optional overall wait upper-bound in seconds. Default: no hard limit (event-driven).",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=None,
        help="Deprecated alias for --max-wait.",
    )
    parser.add_argument(
        "--max-idle-secs",
        type=int,
        default=None,
        help="Optional max idle (no new logs) before fail-fast.",
    )
    parser.add_argument(
        "--webhook-url",
        default=webhook_url_default,
        help="Webhook URL.",
    )
    parser.add_argument(
        "--log-file",
        default=os.environ.get("OMNI_CHANNEL_LOG_FILE", ".run/logs/xiuxian-daochang-webhook.log"),
        help="Runtime log file path.",
    )
    parser.add_argument(
        "--chat-id",
        type=int,
        default=None,
        help="Synthetic Telegram chat id (auto-infer from logs when omitted).",
    )
    parser.add_argument(
        "--user-id",
        type=int,
        default=None,
        help="Synthetic Telegram user id (auto-infer from logs when omitted).",
    )
    parser.add_argument(
        "--username",
        default=os.environ.get("OMNI_TEST_USERNAME"),
        help="Synthetic Telegram username for allowlist checks (e.g. `tao3k`).",
    )
    parser.add_argument(
        "--chat-title",
        default=os.environ.get("OMNI_TEST_CHAT_TITLE"),
        help=(
            "Optional synthetic Telegram chat title to include in payload "
            "(useful for chat_id/chat_title log mapping checks)."
        ),
    )
    parser.add_argument(
        "--thread-id",
        type=int,
        default=None,
        help="Synthetic Telegram thread/topic id.",
    )
    parser.add_argument(
        "--session-partition",
        default=os.environ.get("OMNI_TEST_SESSION_PARTITION"),
        help=(
            "Optional session partition mode hint "
            "(`chat`, `chat_user`, `user`, `chat_thread_user`) "
            "used for strict session-key validation."
        ),
    )
    parser.add_argument(
        "--secret-token",
        default=os.environ.get("TELEGRAM_WEBHOOK_SECRET"),
        help="Webhook secret token header value.",
    )
    parser.add_argument(
        "--no-follow",
        action="store_true",
        help="Disable live log streaming while waiting.",
    )
    parser.add_argument(
        "--native-tools-only",
        action="store_true",
        help=(
            "Enable native-tools-only assertions: require native tool dispatch success "
            "and reject tool-runtime/Zhenfa dispatch in this probe."
        ),
    )
    parser.add_argument(
        "--image-url",
        default=os.environ.get("OMNI_TEST_IMAGE_URL"),
        help=(
            "Optional image URL or data URI to inject as multimodal marker "
            "(appends [IMAGE:...] to the synthetic prompt)."
        ),
    )
    parser.add_argument(
        "--image-file",
        default=os.environ.get("OMNI_TEST_IMAGE_FILE"),
        help=(
            "Optional local image file path; converted to data URI and injected "
            "as multimodal marker. Mutually exclusive with --image-url."
        ),
    )
