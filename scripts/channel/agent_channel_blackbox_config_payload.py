#!/usr/bin/env python3
"""Payload builders for agent channel blackbox."""

from __future__ import annotations

import json
import time


def build_update_payload(
    update_id: int,
    chat_id: int,
    user_id: int,
    username: str | None,
    chat_title: str | None,
    text: str,
    thread_id: int | None,
) -> str:
    """Build one synthetic Telegram webhook update payload."""
    from_user: dict[str, object] = {"id": user_id, "is_bot": False, "first_name": "BlackboxProbe"}
    if username:
        from_user["username"] = username
    chat: dict[str, object] = {"id": chat_id, "type": "private" if chat_id > 0 else "group"}
    if chat_title:
        chat["title"] = chat_title
    message: dict[str, object] = {
        "message_id": update_id % 2_000_000_000,
        "date": int(time.time()),
        "text": text,
        "chat": chat,
        "from": from_user,
    }
    if thread_id is not None:
        message["message_thread_id"] = thread_id
    payload = {"update_id": update_id, "message": message}
    return json.dumps(payload, ensure_ascii=False)


def build_probe_message(prompt: str, trace_id: str, image_url: str | None = None) -> str:
    """Build probe text preserving slash-command exactness and optional image marker."""
    return build_probe_message_with_image(prompt, trace_id, image_url=image_url)


def build_probe_message_with_image(
    prompt: str,
    trace_id: str,
    image_url: str | None,
) -> str:
    """Build probe text and optionally append a multimodal image marker."""
    base_message = prompt if prompt.lstrip().startswith("/") else f"[{trace_id}] {prompt}"

    if not image_url:
        return base_message
    return f"{base_message} [IMAGE:{image_url}]"
