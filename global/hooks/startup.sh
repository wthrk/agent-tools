#!/bin/bash
set -euo pipefail
# SessionStart: agent-tools 更新チェック + jj 検出

_AT_LOG_DIR="${HOME}/.agent-tools/logs"
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

# .jj 検出時は jj ワークフロールールを注入
if [[ -d "${cwd}/.jj" ]]; then
    cat <<'EOF'
{
  "hookSpecificOutput": {
    "hookEventName": "SessionStart",
    "additionalContext": "jj detected. Use /jj skill for version control operations."
  }
}
EOF
else
    echo 'OK'
fi
exit 0
