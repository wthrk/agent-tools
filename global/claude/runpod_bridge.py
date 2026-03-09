#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime
import json
import os
import sys
import urllib.error
import urllib.request
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any, Dict, List

LOG_FILE = "/tmp/runpod_bridge.log"
DEFAULT_UPSTREAM_MODEL = os.environ.get("RUNPOD_BRIDGE_MODEL", "Qwen/Qwen2.5-14B-Instruct")


def log_line(message: str) -> None:
    ts = datetime.datetime.utcnow().isoformat() + "Z"
    try:
        with open(LOG_FILE, "a", encoding="utf-8") as f:
            f.write(f"[{ts}] {message}\n")
    except Exception:
        pass


def anthropic_text_from_content(content: Any) -> str:
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        parts: List[str] = []
        for item in content:
            if isinstance(item, dict) and item.get("type") == "text":
                text = item.get("text")
                if isinstance(text, str):
                    parts.append(text)
        return "\n".join(parts)
    return ""


def anthropic_to_openai_messages(messages: Any) -> List[Dict[str, str]]:
    if not isinstance(messages, list):
        return []
    out: List[Dict[str, str]] = []
    for msg in messages:
        if not isinstance(msg, dict):
            continue
        role = str(msg.get("role", "user"))
        content = anthropic_text_from_content(msg.get("content", ""))
        out.append({"role": role, "content": content})
    return out


def flatten_messages_text(messages: Any) -> str:
    if not isinstance(messages, list):
        return ""
    chunks: List[str] = []
    for msg in messages:
        if not isinstance(msg, dict):
            continue
        chunks.append(anthropic_text_from_content(msg.get("content", "")))
    return "\n".join([c for c in chunks if c])


def resolve_upstream_model(requested: Any) -> str:
    if isinstance(requested, str):
        lowered = requested.lower()
        if lowered.startswith("qwen/") or lowered.startswith("meta-llama/"):
            return requested
        if "sonnet" in lowered or "opus" in lowered or "haiku" in lowered or lowered.startswith("claude"):
            return DEFAULT_UPSTREAM_MODEL
        return requested
    return DEFAULT_UPSTREAM_MODEL


class Handler(BaseHTTPRequestHandler):
    upstream_base = ""
    upstream_token = ""

    def _send_json(self, status: int, payload: Any) -> None:
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _read_json(self) -> Dict[str, Any]:
        try:
            length = int(self.headers.get("content-length", "0"))
        except ValueError:
            length = 0
        raw = self.rfile.read(length) if length > 0 else b"{}"
        try:
            value = json.loads(raw.decode("utf-8"))
            if isinstance(value, dict):
                return value
        except Exception:
            pass
        return {}

    def do_GET(self) -> None:  # noqa: N802
        route = urllib.parse.urlsplit(self.path).path
        log_line(f"GET {route}")
        if route == "/healthz":
            self._send_json(200, {"status": "ok"})
            return
        if route == "/v1/models":
            url = Handler.upstream_base.rstrip("/") + "/v1/models"
            req = urllib.request.Request(
                url,
                method="GET",
                headers={"Authorization": f"Bearer {Handler.upstream_token}"},
            )
            try:
                with urllib.request.urlopen(req, timeout=60) as resp:
                    body = resp.read().decode("utf-8", errors="replace")
                    parsed = json.loads(body)
                    self._send_json(resp.status, parsed)
                    return
            except urllib.error.HTTPError as e:
                body = e.read().decode("utf-8", errors="replace")
                log_line(f"models_http_error code={e.code} body={body[:500]}")
                self._send_json(e.code, {"error": body})
                return
            except Exception as e:
                log_line(f"models_exception error={e}")
                self._send_json(502, {"error": str(e)})
                return
        self._send_json(404, {"error": "not found"})

    def do_POST(self) -> None:  # noqa: N802
        route = urllib.parse.urlsplit(self.path).path
        log_line(f"POST {route}")
        if route == "/v1/messages/count_tokens":
            req = self._read_json()
            text = flatten_messages_text(req.get("messages", []))
            approx = max(1, len(text) // 4)
            self._send_json(200, {"input_tokens": approx})
            return

        if route != "/v1/messages":
            self._send_json(404, {"error": "not found"})
            return

        req = self._read_json()
        requested_model = req.get("model")
        model = resolve_upstream_model(requested_model)
        log_line(f"messages model_requested={requested_model} model_upstream={model}")
        payload = {
            "model": model,
            "messages": anthropic_to_openai_messages(req.get("messages", [])),
            "max_tokens": req.get("max_tokens", 1024),
        }
        if "temperature" in req:
            payload["temperature"] = req["temperature"]
        log_line(
            "payload max_tokens=%s messages=%s first=%s"
            % (
                payload.get("max_tokens"),
                len(payload.get("messages", [])),
                (payload.get("messages", [{}])[0].get("content", "")[:120] if payload.get("messages") else ""),
            )
        )

        url = Handler.upstream_base.rstrip("/") + "/v1/chat/completions"
        data = json.dumps(payload).encode("utf-8")
        upstream_req = urllib.request.Request(
            url,
            data=data,
            method="POST",
            headers={
                "Content-Type": "application/json",
                "Authorization": f"Bearer {Handler.upstream_token}",
            },
        )

        try:
            with urllib.request.urlopen(upstream_req, timeout=120) as resp:
                body = resp.read().decode("utf-8", errors="replace")
                parsed = json.loads(body)
                if not isinstance(parsed, dict):
                    self._send_json(502, {"error": "invalid upstream response"})
                    return
        except urllib.error.HTTPError as e:
            body = e.read().decode("utf-8", errors="replace")
            log_line(f"chat_http_error code={e.code} body={body[:500]}")
            self._send_json(e.code, {"error": body})
            return
        except Exception as e:
            log_line(f"chat_exception error={e}")
            self._send_json(502, {"error": str(e)})
            return

        choice = (parsed.get("choices") or [{}])[0]
        message = choice.get("message") if isinstance(choice, dict) else {}
        usage = parsed.get("usage") if isinstance(parsed.get("usage"), dict) else {}
        text = message.get("content") if isinstance(message, dict) else ""
        self._send_json(
            200,
            {
                "id": parsed.get("id", "msg_proxy"),
                "type": "message",
                "role": "assistant",
                "model": model,
                "content": [{"type": "text", "text": text or ""}],
                "stop_reason": "end_turn",
                "stop_sequence": None,
                "usage": {
                    "input_tokens": usage.get("prompt_tokens", 0),
                    "output_tokens": usage.get("completion_tokens", 0),
                },
            },
        )

    def log_message(self, fmt: str, *args: Any) -> None:
        return


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--upstream-base", required=True)
    parser.add_argument("--upstream-token", required=True)
    args = parser.parse_args()

    Handler.upstream_base = args.upstream_base
    Handler.upstream_token = args.upstream_token
    server = ThreadingHTTPServer(("127.0.0.1", args.port), Handler)
    server.serve_forever()
    return 0


if __name__ == "__main__":
    sys.exit(main())
