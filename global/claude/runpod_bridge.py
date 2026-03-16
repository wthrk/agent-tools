#!/usr/bin/env python3
from __future__ import annotations

import argparse
import datetime
import json
import os
import re
import threading
import urllib.error
import urllib.parse
import urllib.request
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Any, Dict, List, Optional, Tuple

LOG_FILE = os.environ.get("RUNPOD_BRIDGE_LOG_FILE", "/tmp/runpod_bridge.log")
DEFAULT_UPSTREAM_MODEL = os.environ.get("RUNPOD_BRIDGE_MODEL", "Qwen/Qwen2.5-14B-Instruct")
REQ_ID = 0
REQ_ID_LOCK = threading.Lock()


def log_line(message: str) -> None:
    payload = {
        "ts": datetime.datetime.utcnow().isoformat() + "Z",
        "msg": message,
    }
    try:
        with open(LOG_FILE, "a", encoding="utf-8") as handle:
            handle.write(json.dumps(payload, ensure_ascii=False) + "\n")
    except OSError:
        pass


def next_request_id() -> int:
    global REQ_ID
    with REQ_ID_LOCK:
        REQ_ID += 1
        return REQ_ID


def read_tail_lines(path: str, tail: int) -> List[str]:
    tail = max(1, min(tail, 1000))
    try:
        with open(path, "rb") as handle:
            handle.seek(0, os.SEEK_END)
            position = handle.tell()
            chunk = bytearray()
            newline_count = 0

            while position > 0 and newline_count <= tail:
                read_size = min(4096, position)
                position -= read_size
                handle.seek(position)
                data = handle.read(read_size)
                chunk[:0] = data
                newline_count = chunk.count(b"\n")
    except OSError:
        return []
    lines = chunk.decode("utf-8", errors="replace").splitlines()
    return lines[-tail:]


def anthropic_text_from_content(content: Any) -> str:
    if isinstance(content, str):
        return content
    if not isinstance(content, list):
        return ""

    parts: List[str] = []
    for item in content:
        if not isinstance(item, dict):
            continue
        item_type = item.get("type")
        if item_type == "text":
            text = item.get("text")
            if isinstance(text, str):
                parts.append(text)
        elif item_type == "tool_result":
            tool_use_id = item.get("tool_use_id", "unknown")
            rendered = anthropic_text_from_content(item.get("content", ""))
            parts.append(f"Tool result for {tool_use_id}:\n{rendered}")
    return "\n".join(part for part in parts if part)


def anthropic_to_openai_messages(messages: Any) -> List[Dict[str, str]]:
    if not isinstance(messages, list):
        return []
    out: List[Dict[str, str]] = []
    for message in messages:
        if not isinstance(message, dict):
            continue
        role = message.get("role")
        if role not in ("user", "assistant", "system"):
            continue
        out.append({"role": role, "content": anthropic_text_from_content(message.get("content", ""))})
    return out


def flatten_messages_text(messages: Any) -> str:
    return "\n".join(
        message.get("content", "")
        for message in anthropic_to_openai_messages(messages)
        if isinstance(message.get("content"), str)
    )


def resolve_upstream_model(requested: Any) -> str:
    if isinstance(requested, str) and requested.strip():
        return requested
    return DEFAULT_UPSTREAM_MODEL


def clamp_max_tokens(value: Any) -> int:
    try:
        token_count = int(value)
    except (TypeError, ValueError):
        return 1024
    return max(1, min(token_count, 2048))


def anthropic_tools_to_openai(tools: Any) -> List[Dict[str, Any]]:
    if not isinstance(tools, list):
        return []
    out: List[Dict[str, Any]] = []
    for tool in tools:
        if not isinstance(tool, dict):
            continue
        name = tool.get("name")
        if not isinstance(name, str) or not name:
            continue
        description = tool.get("description", "")
        input_schema = tool.get("input_schema", {})
        if not isinstance(input_schema, dict):
            input_schema = {"type": "object", "properties": {}}
        out.append(
            {
                "type": "function",
                "function": {
                    "name": name,
                    "description": description if isinstance(description, str) else "",
                    "parameters": input_schema,
                },
            }
        )
    return out


def render_tools_for_prompt(tools: Any) -> str:
    if not isinstance(tools, list) or not tools:
        return ""
    lines = [
        "You are operating in Claude-compatible tool mode.",
        "Return JSON only.",
        "Use one of these shapes:",
        "{\"content\":[{\"type\":\"text\",\"text\":\"...\"}]}",
        "{\"content\":[{\"type\":\"text\",\"text\":\"...\"},{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"name\":\"<tool_name>\",\"input\":{...}}]}",
        "{\"content\":[{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"name\":\"<tool_name>\",\"input\":{...}}]}",
        "Do not wrap JSON in markdown fences.",
        "Use declared tools exactly and keep input_schema exact.",
        "Available tools:",
    ]
    for tool in tools:
        if not isinstance(tool, dict):
            continue
        schema = json.dumps(tool.get("input_schema", {}), ensure_ascii=False, sort_keys=True)
        lines.append(f"- {tool.get('name', '')}: {tool.get('description', '')}")
        lines.append(f"  input_schema={schema}")
    return "\n".join(lines)


def build_openai_messages(req: Dict[str, Any]) -> List[Dict[str, str]]:
    messages = anthropic_to_openai_messages(req.get("messages", []))
    tools_prompt = render_tools_for_prompt(req.get("tools"))
    if tools_prompt:
        return [{"role": "system", "content": tools_prompt}, *messages]
    return messages


def repair_common_json_issues(candidate: str) -> Optional[str]:
    repaired = re.sub(r",\s*([}\]])", r"\1", candidate)
    repaired = re.sub(r'("([^"\\]|\\.)*")\s*\.\s*', r"\1: ", repaired)
    if repaired == candidate:
        return None
    return repaired


def parse_tool_arguments(raw: Any) -> Dict[str, Any]:
    if isinstance(raw, dict):
        return raw
    if not isinstance(raw, str):
        return {}
    try:
        parsed = json.loads(raw)
    except json.JSONDecodeError:
        repaired = repair_common_json_issues(raw)
        if repaired is None:
            return {}
        try:
            parsed = json.loads(repaired)
        except json.JSONDecodeError:
            return {}
    return parsed if isinstance(parsed, dict) else {}


def schema_type_matches(value: Any, expected: str) -> bool:
    if expected == "string":
        return isinstance(value, str)
    if expected == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if expected == "number":
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    if expected == "boolean":
        return isinstance(value, bool)
    if expected == "object":
        return isinstance(value, dict)
    if expected == "array":
        return isinstance(value, list)
    if expected == "null":
        return value is None
    return True


def validate_tool_input(schema: Any, tool_input: Any) -> Optional[str]:
    if not isinstance(schema, dict):
        return None
    schema_type = schema.get("type")
    if isinstance(schema_type, str) and not schema_type_matches(tool_input, schema_type):
        return f"input must be {schema_type}"
    if not isinstance(tool_input, dict):
        return None

    required = schema.get("required")
    if isinstance(required, list):
        for key in required:
            if isinstance(key, str) and key not in tool_input:
                return f"missing required key: {key}"

    properties = schema.get("properties")
    if isinstance(properties, dict):
        for key, value in tool_input.items():
            if key not in properties:
                return f"unexpected key: {key}"
            prop_schema = properties.get(key)
            if isinstance(prop_schema, dict):
                prop_type = prop_schema.get("type")
                if isinstance(prop_type, str) and not schema_type_matches(value, prop_type):
                    return f"key {key} must be {prop_type}"
    return None


def content_from_openai_message(message: Any, req_tools: Any) -> Tuple[Optional[List[Dict[str, Any]]], Optional[str]]:
    if not isinstance(message, dict):
        return None, "upstream message must be an object"
    tools_by_name: Dict[str, Dict[str, Any]] = {}
    if isinstance(req_tools, list):
        for tool in req_tools:
            if isinstance(tool, dict):
                name = tool.get("name")
                if isinstance(name, str) and name:
                    tools_by_name[name] = tool

    normalized: List[Dict[str, Any]] = []
    text = message.get("content")
    if isinstance(text, str) and text:
        normalized.append({"type": "text", "text": text})

    tool_calls = message.get("tool_calls")
    if isinstance(tool_calls, list):
        for tool_call in tool_calls:
            if not isinstance(tool_call, dict):
                return None, "tool_call must be an object"
            function = tool_call.get("function")
            if not isinstance(function, dict):
                return None, "tool_call.function must be an object"
            tool_name = function.get("name")
            if not isinstance(tool_name, str) or not tool_name:
                return None, "tool_call.function.name must be a string"
            tool_def = tools_by_name.get(tool_name)
            if tool_def is None:
                return None, f"unknown tool: {tool_name}"
            tool_input = parse_tool_arguments(function.get("arguments"))
            validation_error = validate_tool_input(tool_def.get("input_schema"), tool_input)
            if validation_error is not None:
                return None, f"{tool_name} input invalid: {validation_error}"
            tool_use_id = tool_call.get("id")
            if not isinstance(tool_use_id, str) or not tool_use_id:
                tool_use_id = f"toolu_{next_request_id()}"
            normalized.append(
                {
                    "type": "tool_use",
                    "id": tool_use_id,
                    "name": tool_name,
                    "input": tool_input,
                }
            )
    if not normalized:
        return None, "upstream message had neither text nor tool_calls"
    return normalized, None


def extract_json_candidate(text: str) -> Optional[str]:
    stripped = text.strip()
    if not stripped:
        return None
    if stripped.startswith("```"):
        lines = stripped.splitlines()
        if len(lines) >= 3:
            body = "\n".join(lines[1:-1]).strip()
            if body:
                return body
    starts = [index for index in (stripped.find("{"), stripped.find("[")) if index >= 0]
    if not starts:
        return None
    start = min(starts)
    end = max(stripped.rfind("}"), stripped.rfind("]"))
    if end < start:
        return None
    return stripped[start : end + 1]


def parse_tool_response_text(text: str) -> Optional[Dict[str, Any]]:
    candidate = extract_json_candidate(text)
    if candidate is None:
        return None
    try:
        parsed = json.loads(candidate)
    except json.JSONDecodeError:
        repaired = repair_common_json_issues(candidate)
        if repaired is None:
            return None
        try:
            parsed = json.loads(repaired)
        except json.JSONDecodeError:
            return None
    if isinstance(parsed, dict):
        return parsed
    if isinstance(parsed, list):
        return {"content": parsed}
    return None


def normalize_assistant_content(parsed: Dict[str, Any], req_tools: Any) -> Tuple[Optional[List[Dict[str, Any]]], Optional[str]]:
    raw_content = parsed.get("content")
    if not isinstance(raw_content, list):
        return None, "top-level content must be an array"
    tools_by_name: Dict[str, Dict[str, Any]] = {}
    if isinstance(req_tools, list):
        for tool in req_tools:
            if isinstance(tool, dict):
                name = tool.get("name")
                if isinstance(name, str) and name:
                    tools_by_name[name] = tool
    normalized: List[Dict[str, Any]] = []
    for block in raw_content:
        if not isinstance(block, dict):
            return None, "content blocks must be objects"
        if block.get("type") == "text":
            text = block.get("text", "")
            if not isinstance(text, str):
                text = json.dumps(text, ensure_ascii=False)
            normalized.append({"type": "text", "text": text})
            continue
        if block.get("type") == "tool_use":
            tool_name = block.get("name")
            if not isinstance(tool_name, str) or not tool_name:
                return None, "tool_use.name must be a non-empty string"
            tool_def = tools_by_name.get(tool_name)
            if tool_def is None:
                return None, f"unknown tool: {tool_name}"
            tool_input = block.get("input", {})
            validation_error = validate_tool_input(tool_def.get("input_schema"), tool_input)
            if validation_error is not None:
                return None, f"{tool_name} input invalid: {validation_error}"
            tool_use_id = block.get("id")
            if not isinstance(tool_use_id, str) or not tool_use_id:
                tool_use_id = f"toolu_{next_request_id()}"
            normalized.append(
                {
                    "type": "tool_use",
                    "id": tool_use_id,
                    "name": tool_name,
                    "input": tool_input,
                }
            )
            continue
        return None, "content blocks must be text or tool_use"
    if not normalized:
        return None, "content must not be empty"
    return normalized, None


def stop_reason_for_content(content: List[Dict[str, Any]]) -> str:
    for block in content:
        if block.get("type") == "tool_use":
            return "tool_use"
    return "end_turn"


def make_anthropic_response(response_id: Any, model: str, content: List[Dict[str, Any]], usage: Dict[str, Any]) -> Dict[str, Any]:
    return {
        "id": response_id or "msg_proxy",
        "type": "message",
        "role": "assistant",
        "model": model,
        "content": content,
        "stop_reason": stop_reason_for_content(content),
        "stop_sequence": None,
        "usage": {
            "input_tokens": usage.get("prompt_tokens", 0),
            "output_tokens": usage.get("completion_tokens", 0),
        },
    }


class Handler(BaseHTTPRequestHandler):
    upstream_base = ""
    upstream_token = ""

    def _read_json(self) -> Dict[str, Any]:
        length = int(self.headers.get("Content-Length", "0"))
        raw = self.rfile.read(length) if length > 0 else b"{}"
        try:
            parsed = json.loads(raw.decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError):
            return {}
        return parsed if isinstance(parsed, dict) else {}

    def _send_json(self, code: int, payload: Dict[str, Any]) -> None:
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self) -> None:  # noqa: N802
        route = urllib.parse.urlsplit(self.path).path
        query = urllib.parse.parse_qs(urllib.parse.urlsplit(self.path).query)
        log_line(f"GET {route}")
        if route == "/healthz":
            self._send_json(200, {"status": "ok"})
            return
        if route == "/debug/logs":
            tail = 200
            if "tail" in query and query["tail"]:
                try:
                    tail = int(query["tail"][0])
                except ValueError:
                    tail = 200
            self._send_json(200, {"log_file": LOG_FILE, "lines": read_tail_lines(LOG_FILE, tail)})
            return
        if route == "/v1/models":
            req = urllib.request.Request(
                Handler.upstream_base.rstrip("/") + "/v1/models",
                headers={"Authorization": f"Bearer {Handler.upstream_token}"},
            )
            try:
                with urllib.request.urlopen(req, timeout=30) as response:
                    payload = json.loads(response.read().decode("utf-8", errors="replace"))
            except Exception as exc:  # noqa: BLE001
                self._send_json(502, {"error": str(exc)})
                return
            if isinstance(payload, dict):
                self._send_json(200, payload)
                return
            self._send_json(502, {"error": "invalid upstream response"})
            return
        self._send_json(404, {"error": "not found"})

    def do_POST(self) -> None:  # noqa: N802
        route = urllib.parse.urlsplit(self.path).path
        request_id = next_request_id()
        log_line(f"rid={request_id} POST {route}")

        if route == "/v1/messages/count_tokens":
            req = self._read_json()
            approx = max(1, len(flatten_messages_text(req.get("messages", []))) // 4)
            self._send_json(200, {"input_tokens": approx})
            return

        if route != "/v1/messages":
            self._send_json(404, {"error": "not found"})
            return

        req = self._read_json()
        requested_model = req.get("model")
        model = resolve_upstream_model(requested_model)
        openai_tools = anthropic_tools_to_openai(req.get("tools"))
        payload: Dict[str, Any] = {
            "model": model,
            "messages": anthropic_to_openai_messages(req.get("messages", [])),
            "max_tokens": clamp_max_tokens(req.get("max_tokens", 1024)),
        }
        if openai_tools:
            payload["tools"] = openai_tools
            payload["tool_choice"] = "auto"
        elif req.get("tools"):
            payload["messages"] = build_openai_messages(req)
        if "temperature" in req:
            payload["temperature"] = req["temperature"]

        url = Handler.upstream_base.rstrip("/") + "/v1/chat/completions"
        validation_error: Optional[str] = None
        for attempt in range(2):
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
                with urllib.request.urlopen(upstream_req, timeout=120) as response:
                    parsed = json.loads(response.read().decode("utf-8", errors="replace"))
            except urllib.error.HTTPError as exc:
                body = exc.read().decode("utf-8", errors="replace")
                log_line(f"rid={request_id} chat_http_error code={exc.code} body={body[:500]}")
                self._send_json(exc.code, {"error": body})
                return
            except Exception as exc:  # noqa: BLE001
                log_line(f"rid={request_id} chat_exception error={exc}")
                self._send_json(502, {"error": str(exc)})
                return

            if not isinstance(parsed, dict):
                self._send_json(502, {"error": "invalid upstream response"})
                return

            choice = (parsed.get("choices") or [{}])[0]
            message = choice.get("message") if isinstance(choice, dict) else {}
            usage = parsed.get("usage") if isinstance(parsed.get("usage"), dict) else {}
            text = message.get("content") if isinstance(message, dict) else ""

            if not req.get("tools"):
                self._send_json(
                    200,
                    make_anthropic_response(
                        parsed.get("id", "msg_proxy"),
                        model,
                        [{"type": "text", "text": text or ""}],
                        usage,
                    ),
                )
                return

            normalized_content, native_error = content_from_openai_message(message, req.get("tools"))
            if normalized_content is not None:
                self._send_json(
                    200,
                    make_anthropic_response(parsed.get("id", "msg_proxy"), model, normalized_content, usage),
                )
                return

            if openai_tools and attempt == 0:
                payload.pop("tools", None)
                payload.pop("tool_choice", None)
                payload["messages"] = build_openai_messages(req)
                continue

            tool_payload = parse_tool_response_text(text or "")
            if tool_payload is not None:
                normalized_content, validation_error = normalize_assistant_content(tool_payload, req.get("tools"))
                if normalized_content is not None:
                    self._send_json(
                        200,
                        make_anthropic_response(parsed.get("id", "msg_proxy"), model, normalized_content, usage),
                    )
                    return
            else:
                validation_error = native_error or "response was not valid JSON matching the Claude content schema"

            if attempt == 0:
                retry_instruction = (
                    "Your previous response was invalid for Claude-compatible tool mode.\n"
                    f"Validation error: {validation_error}\n"
                    "Reply again with JSON only.\n"
                    "{\"content\":[{\"type\":\"text\",\"text\":\"...\"}]}\n"
                    "or\n"
                    "{\"content\":[{\"type\":\"text\",\"text\":\"...\"},{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"name\":\"<tool_name>\",\"input\":{...}}]}"
                )
                payload["messages"] = [*payload.get("messages", []), {"role": "system", "content": retry_instruction}]
                continue

        self._send_json(502, {"error": "tool response validation failed", "details": validation_error or "unknown validation error"})

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
    log_line(f"bridge_started port={args.port} upstream={args.upstream_base}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        return 0
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
