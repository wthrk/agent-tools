#!/bin/bash
# hooks/skill-simple-eval-hook.sh
# UserPromptSubmit hook - light reminder to check skills
# stdin: user's prompt
# stdout: modified prompt

PROMPT=$(cat)

cat << EOF
Note: Check if any available skills might help with this task.

$PROMPT
EOF
