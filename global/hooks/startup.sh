#!/bin/bash
set -euo pipefail
# SessionStart: agent-tools 更新チェック + jj 検出

_AT_LOG_DIR="${HOME}/.agent-tools/logs"
mkdir -p "${_AT_LOG_DIR}"

if command -v agent-tools &>/dev/null; then
    agent-tools startup >/dev/null 2>>"${_AT_LOG_DIR}/startup-hook.log" || true
fi

# .jj 検出時は jj ワークフロールールを注入
if [[ -d "${PWD}/.jj" ]]; then
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
