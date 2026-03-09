from __future__ import annotations

import json
import os
import urllib.error
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any, Dict, List, Optional, Tuple, Union

VLLM_INTERNAL_PORT = os.getenv("VLLM_INTERNAL_PORT", "8001")
PROXY_PORT = int(os.getenv("PROXY_PORT", "8000"))
STATUS_FILE = os.getenv("STATUS_FILE", "/workspace/runpod-status.json")
VLLM_BASE = f"http://127.0.0.1:{VLLM_INTERNAL_PORT}/v1"


JsonObj = Dict[str, Any]
JsonList = List[Any]
JsonLike = Union[JsonObj, JsonList, str]


def read_state() -> JsonObj:
    try:
        with open(STATUS_FILE, encoding="utf-8") as f:
            value = json.load(f)
        if isinstance(value, dict):
            return value
    except Exception:
        pass
    return {"phase": "unknown", "ready": False, "message": "status file is not available"}


def anthropic_text_from_content(content: Any) -> str:
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        parts: list[str] = []
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
        role = msg.get("role", "user")
        content = anthropic_text_from_content(msg.get("content", ""))
        out.append({"role": str(role), "content": content})
    return out


def http_json(
    method: str, url: str, payload: Optional[Any]
) -> Tuple[int, str, JsonLike]:
    data: Optional[bytes] = None
    headers = {"content-type": "application/json"}
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            raw = resp.read()
            text = raw.decode("utf-8", errors="replace")
            return resp.status, text, json.loads(text)
    except urllib.error.HTTPError as e:
        raw = e.read()
        text = raw.decode("utf-8", errors="replace")
        try:
            parsed = json.loads(text)
        except Exception:
            parsed = text
        return e.code, text, parsed
    except Exception as e:
        return 502, str(e), {"error": str(e)}


class Handler(BaseHTTPRequestHandler):
    def _send_json(self, status: int, payload: Any) -> None:
        body = json.dumps(payload).encode("utf-8")
        self.send_response(status)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def _read_json_body(self) -> dict[str, Any]:
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
        if self.path == "/healthz":
            self._send_json(200, {"status": "ok"})
            return
        if self.path == "/readyz":
            state = read_state()
            self._send_json(200 if state.get("ready") is True else 503, state)
            return
        state = read_state()
        if state.get("ready") is not True:
            self._send_json(503, state)
            return

        path = self.path
        if path.startswith("/openai/v1/"):
            path = "/v1/" + path[len("/openai/v1/") :]
        if not path.startswith("/v1/"):
            self._send_json(404, {"error": "not found"})
            return
        status, _, payload = http_json("GET", f"{VLLM_BASE}/{path[len('/v1/'):]}", None)
        self._send_json(status, payload)

    def do_POST(self) -> None:  # noqa: N802
        state = read_state()
        if state.get("ready") is not True:
            self._send_json(503, state)
            return

        req = self._read_json_body()
        if self.path == "/v1/messages":
            model = req.get("model")
            max_tokens = req.get("max_tokens", 1024)
            payload = {
                "model": model,
                "messages": anthropic_to_openai_messages(req.get("messages", [])),
                "max_tokens": max_tokens,
            }
            if "temperature" in req:
                payload["temperature"] = req["temperature"]
            status, text, data = http_json("POST", f"{VLLM_BASE}/chat/completions", payload)
            if status >= 500:
                self._send_json(status, {"error": text})
                return
            if not isinstance(data, dict):
                self._send_json(502, {"error": "invalid response from vllm"})
                return
            choice = (data.get("choices") or [{}])[0]
            message = choice.get("message") if isinstance(choice, dict) else {}
            usage = data.get("usage") if isinstance(data.get("usage"), dict) else {}
            assistant_text = (
                message.get("content") if isinstance(message, dict) else ""
            ) or ""
            self._send_json(
                200,
                {
                    "id": data.get("id", "msg_proxy"),
                    "type": "message",
                    "role": "assistant",
                    "model": model,
                    "content": [{"type": "text", "text": assistant_text}],
                    "stop_reason": "end_turn",
                    "stop_sequence": None,
                    "usage": {
                        "input_tokens": usage.get("prompt_tokens", 0),
                        "output_tokens": usage.get("completion_tokens", 0),
                    },
                },
            )
            return

        path = self.path
        if path.startswith("/openai/v1/"):
            path = "/v1/" + path[len("/openai/v1/") :]
        if not path.startswith("/v1/"):
            self._send_json(404, {"error": "not found"})
            return
        status, _, payload = http_json("POST", f"{VLLM_BASE}/{path[len('/v1/'):]}", req)
        self._send_json(status, payload)

    def log_message(self, fmt: str, *args: Any) -> None:
        return


def main() -> None:
    server = ThreadingHTTPServer(("0.0.0.0", PROXY_PORT), Handler)
    server.serve_forever()


if __name__ == "__main__":
    main()
