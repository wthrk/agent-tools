#!/bin/bash
set -euo pipefail

input=$(cat)
command=$(echo "$input" | jq -r '.tool_input.command // empty')

# Extract first word of the command (the actual command being run)
first_word=$(echo "$command" | awk '{print $1}')

# Block if the command itself is git
if [[ "$first_word" == "git" ]]; then
    cat >&2 <<'EOF'
gitは使用禁止です。代わりにjjを使用してください:
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
gitは使用禁止です。代わりにjjを使用してください:
  git status  → jj status
  git diff    → jj diff
  git log     → jj log
  git commit  → jj commit
  git push    → jj git push
EOF
    exit 2
fi

exit 0
