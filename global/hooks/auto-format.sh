#!/bin/bash
# auto-format.sh - Edit/Write後にファイルを自動フォーマット
# PostToolUse (Edit|Write) フック用

set -euo pipefail

if ! command -v jq &> /dev/null; then
    exit 0
fi

input=$(cat)
file_path=$(echo "$input" | jq -r '.tool_input.file_path // empty' 2>/dev/null || echo "")

if [[ -z "$file_path" ]] || [[ ! -f "$file_path" ]]; then
    exit 0
fi

case "$file_path" in
    *.rs)
        rustfmt "$file_path" 2>/dev/null || true
        ;;
esac
