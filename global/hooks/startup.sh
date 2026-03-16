#!/bin/bash
set -euo pipefail
# SessionStart: agent-tools 更新チェック + jj 検出

_AT_HOME="${AGENT_TOOLS_HOME:-${HOME}/.agent-tools}"
_AT_LOG_DIR="${_AT_HOME}/logs"
mkdir -p "${_AT_LOG_DIR}"

if command -v agent-tools &>/dev/null; then
    agent-tools startup >/dev/null 2>>"${_AT_LOG_DIR}/startup-hook.log" || true
fi

# セッションの cwd を stdin(JSON) から取得。未取得時は PWD を使う。
stdin_payload="$(cat || true)"
cwd="${PWD}"
if [[ -n "${stdin_payload}" ]] && command -v jq &>/dev/null; then
    parsed_cwd="$(printf '%s' "${stdin_payload}" | jq -r '.cwd // empty' 2>/dev/null || true)"
    if [[ -n "${parsed_cwd}" ]]; then
        cwd="${parsed_cwd}"
    fi
fi

# 追加コンテキストを必要に応じて積み上げる
context_lines=()
current_profile_file="${_AT_HOME}/.local/state/current.json"
runpod_active=0

if [[ -f "${current_profile_file}" ]] && command -v jq &>/dev/null; then
    current_claude_profile="$(jq -r '.claude // empty' "${current_profile_file}" 2>/dev/null || true)"
    if [[ "${current_claude_profile}" == "runpod" ]]; then
        runpod_active=1
    fi
fi

# .jj 検出時は jj ワークフロールールを注入
if [[ -d "${cwd}/.jj" ]]; then
    context_lines+=("jj detected. Use /jj skill for version control operations.")
fi

if [[ ${runpod_active} -eq 1 ]]; then
    context_lines+=("RunPod Claude-compatible tool mode is active. Emit structured content blocks, not prose-only tool descriptions.")
    context_lines+=("Assistant tool call shape: {\"role\":\"assistant\",\"content\":[{\"type\":\"text\",\"text\":\"...\"},{\"type\":\"tool_use\",\"id\":\"toolu_...\",\"name\":\"<tool_name>\",\"input\":{...}}]}.")
    context_lines+=("Tool result shape: {\"role\":\"user\",\"content\":[{\"type\":\"tool_result\",\"tool_use_id\":\"toolu_...\",\"content\":\"...\"}]}.")
    context_lines+=("Use the declared tool name exactly. The input object must match the tool input_schema exactly: required keys present, correct JSON types, no extra keys.")
    context_lines+=("If multiple tools are needed, emit multiple tool_use blocks in one assistant message. If no tool is needed, return normal assistant text only.")
fi

expected_base_url_file="${HOME}/.claude/runpod_expected_anthropic_base_url"
if [[ -f "${expected_base_url_file}" ]]; then
    expected_base_url="$(cat "${expected_base_url_file}" 2>/dev/null || true)"
    expected_base_url="${expected_base_url//$'\n'/}"
    current_base_url="${ANTHROPIC_BASE_URL:-}"
    if [[ -n "${expected_base_url}" ]] && [[ "${current_base_url}" != "${expected_base_url}" ]]; then
        context_lines+=("RunPod profile is active but ANTHROPIC_BASE_URL is not synced. Run: source ~/.claude/runpod.env")
    fi
fi

if [[ ${#context_lines[@]} -gt 0 ]]; then
    joined="$(printf '%s\n' "${context_lines[@]}")"
    if command -v jq &>/dev/null; then
        jq -n --arg ctx "${joined}" \
          '{"hookSpecificOutput":{"hookEventName":"SessionStart","additionalContext":$ctx}}'
    else
        escaped="${joined//\\/\\\\}"
        escaped="${escaped//\"/\\\"}"
        escaped="${escaped//$'\n'/\\n}"
        cat <<EOF
{
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": "${escaped}"
  }
}
EOF
    fi
fi
exit 0
