#!/bin/bash
# detect-jj.sh - .jj検出時にjjワークフロールールを注入
# UserPromptSubmitフック用

set -euo pipefail

# jqがない場合はスキップ
if ! command -v jq &> /dev/null; then
    echo '{}'
    exit 0
fi

input=$(cat)
cwd=$(echo "$input" | jq -r '.cwd // empty' 2>/dev/null || echo "")

# cwdが空または.jjがない場合はスキップ
if [[ -z "$cwd" ]] || [[ ! -d "$cwd/.jj" ]]; then
    echo '{}'
    exit 0
fi

# .jj検出時はjjワークフロールールを注入
cat <<'EOF'
{
  "hookSpecificOutput": {
    "hookEventName": "UserPromptSubmit",
    "additionalContext": "## jj workflow (.jj detected)\nParallel-first: all work = one of N parallel tasks.\n- Check: jj status before work\n- Switch: jj edit <rev>\n- New: jj new main -m \"desc\"\n- Details: use /jj skill"
  }
}
EOF
