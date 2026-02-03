#!/bin/bash
# block-git.sh - .jjリポジトリでgitコマンドをブロック
# PreToolUse (Bash) フック用

set -euo pipefail

# jqがない場合はスキップ
if ! command -v jq &> /dev/null; then
    exit 0
fi

input=$(cat)
cwd=$(echo "$input" | jq -r '.cwd // empty' 2>/dev/null || echo "")
command=$(echo "$input" | jq -r '.tool_input.command // empty' 2>/dev/null || echo "")

# cwdが空または.jjがない場合はスキップ
if [[ -z "$cwd" ]] || [[ ! -d "$cwd/.jj" ]]; then
    exit 0
fi

# Extract first word of the command (the actual command being run)
first_word=$(echo "$command" | awk '{print $1}')

# Block if the command itself is git
if [[ "$first_word" == "git" ]]; then
    cat >&2 <<'EOF'
gitは使用禁止です（jjリポジトリ検出）。代わりにjjを使用してください:
  git status  → jj status
  git diff    → jj diff
  git log     → jj log
  git commit  → jj commit
  git push    → jj git push
EOF
    exit 2
fi

# Also check for git in compound commands (after && || ; |)
# Remove quoted strings first to avoid false positives
stripped=$(echo "$command" | sed -E "s/'[^']*'//g; s/\"[^\"]*\"//g")

if echo "$stripped" | grep -qE '(&&|\|\||;|\|)\s*git\s'; then
    cat >&2 <<'EOF'
gitは使用禁止です（jjリポジトリ検出）。代わりにjjを使用してください:
  git status  → jj status
  git diff    → jj diff
  git log     → jj log
  git commit  → jj commit
  git push    → jj git push
EOF
    exit 2
fi

exit 0
