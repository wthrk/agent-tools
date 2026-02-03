#!/bin/bash
# block-tail-head.sh - tail/head/grepコマンドをブロック
# PreToolUse (Bash) フック用

set -euo pipefail

# jqがない場合はスキップ
if ! command -v jq &> /dev/null; then
    exit 0
fi

input=$(cat)
command=$(echo "$input" | jq -r '.tool_input.command // empty' 2>/dev/null || echo "")

# Extract first word of the command (the actual command being run)
first_word=$(echo "$command" | awk '{print $1}')

# Block if the command is tail, head, or grep
if [[ "$first_word" == "tail" || "$first_word" == "head" || "$first_word" == "grep" ]]; then
    cat >&2 <<'EOF'
tail/head/grepは使用禁止です。代わりにRead/Grepツールを使用してください。
EOF
    exit 2
fi

# Also check for tail/head/grep in compound commands (after && || ; |)
# Remove quoted strings first to avoid false positives
stripped=$(echo "$command" | sed -E "s/'[^']*'//g; s/\"[^\"]*\"//g")

if echo "$stripped" | awk '/(&&|\|\||;|\|)[[:space:]]*(tail|head|grep)[[:space:]]/ {exit 0} {exit 1}'; then
    cat >&2 <<'EOF'
tail/head/grepは使用禁止です。代わりにRead/Grepツールを使用してください。
EOF
    exit 2
fi

exit 0
