#!/bin/bash
# cmux-notify.sh - タスク完了時にcmux通知を送信
# Stop フック用

set -euo pipefail

if ! command -v cmux &>/dev/null; then
  exit 0
fi

if [ -S /tmp/cmux.sock ] || [ -n "${CMUX_WORKSPACE_ID:-}" ]; then
  echo '{}' | cmux claude-hook stop 2>/dev/null || true
fi
