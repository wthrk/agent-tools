#!/bin/bash
# agent-tools の更新チェック + sync を SessionStart で実行
# stdout は hookSpecificOutput として解析されるため、
# コマンド出力は /dev/null にリダイレクト
if command -v agent-tools &>/dev/null; then
    agent-tools startup >/dev/null 2>&1 || true
fi
echo 'OK'
exit 0
