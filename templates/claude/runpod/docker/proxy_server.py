from __future__ import annotations

import os
from typing import Any

import httpx
from fastapi import FastAPI, HTTPException, Request
from fastapi.responses import JSONResponse

app = FastAPI()
VLLM_INTERNAL_PORT = os.getenv("VLLM_INTERNAL_PORT", "8001")
VLLM_BASE = f"http://127.0.0.1:{VLLM_INTERNAL_PORT}/v1"


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


def anthropic_to_openai_messages(messages: list[dict[str, Any]]) -> list[dict[str, str]]:
    out: list[dict[str, str]] = []
    for msg in messages:
        role = msg.get("role", "user")
        content = anthropic_text_from_content(msg.get("content", ""))
        out.append({"role": role, "content": content})
    return out


@app.get("/healthz")
async def healthz() -> dict[str, str]:
    return {"status": "ok"}


@app.post("/v1/messages")
async def anthropic_messages(req: dict[str, Any]) -> dict[str, Any]:
    model = req.get("model")
    max_tokens = req.get("max_tokens", 1024)
    messages = req.get("messages", [])

    payload = {
        "model": model,
        "messages": anthropic_to_openai_messages(messages),
        "max_tokens": max_tokens,
    }
    if "temperature" in req:
        payload["temperature"] = req["temperature"]

    async with httpx.AsyncClient(timeout=120.0) as client:
        resp = await client.post(f"{VLLM_BASE}/chat/completions", json=payload)

    if resp.status_code >= 500:
        raise HTTPException(status_code=resp.status_code, detail=resp.text)

    data = resp.json()
    choice = (data.get("choices") or [{}])[0]
    message = choice.get("message") or {}
    assistant_text = message.get("content", "")
    usage = data.get("usage") or {}

    return {
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
    }


@app.api_route("/openai/v1/{path:path}", methods=["GET", "POST"])
async def openai_proxy(path: str, request: Request) -> JSONResponse:
    return await _proxy_to_vllm(path, request)


@app.api_route("/v1/{path:path}", methods=["GET", "POST"])
async def v1_proxy(path: str, request: Request) -> JSONResponse:
    if path == "messages":
        raise HTTPException(status_code=405, detail="Use POST /v1/messages")
    return await _proxy_to_vllm(path, request)


async def _proxy_to_vllm(path: str, request: Request) -> JSONResponse:
    method = request.method
    url = f"{VLLM_BASE}/{path}"
    headers = {"content-type": request.headers.get("content-type", "application/json")}
    body = await request.body()
    async with httpx.AsyncClient(timeout=120.0) as client:
        resp = await client.request(method, url, headers=headers, content=body)
    content_type = resp.headers.get("content-type", "application/json")
    if "application/json" in content_type:
        try:
            payload = resp.json()
        except ValueError:
            payload = {"raw": resp.text}
    else:
        payload = {"raw": resp.text}
    return JSONResponse(status_code=resp.status_code, content=payload)
