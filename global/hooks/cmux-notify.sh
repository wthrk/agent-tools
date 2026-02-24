#!/bin/bash
# cmux-notify.sh - タスク完了時にcmux通知を送信
# Stop フック用

set -euo pipefail

# cmuxが利用可能な場合のみ通知
if ! command -v cmux &>/dev/null; then
  exit 0
fi

if [ -S /tmp/cmux.sock ] || [ -n "${CMUX_WORKSPACE_ID:-}" ]; then
  cmux notify \
    --title "Claude Code" \
    --body "タスクが完了しました" \
    --level info 2>/dev/null || true
fi
