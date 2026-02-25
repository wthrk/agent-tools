#!/bin/bash
set -euo pipefail
# agent-tools の更新チェック + sync を SessionStart で実行
# stdout は hookSpecificOutput として解析されるため、
# コマンド出力は /dev/null にリダイレクト
_AT_LOG_DIR="${HOME}/.agent-tools/logs"
mkdir -p "${_AT_LOG_DIR}"

if command -v agent-tools &>/dev/null; then
    agent-tools startup >/dev/null 2>>"${_AT_LOG_DIR}/startup-hook.log" || true
fi
echo 'OK'
exit 0
