#!/bin/bash
# hooks/skill-forced-eval-hook.sh
# UserPromptSubmit hook - forces skill evaluation
# stdin: user's prompt
# stdout: modified prompt

PROMPT=$(cat)

cat << EOF
Before implementing, you MUST:

1. EVALUATE each available skill:
   - List each skill name
   - Write YES or NO
   - Give reason

2. CALL the Skill tool for each YES skill:
   - Do NOT skip this step
   - Evaluation alone is NOT sufficient

3. Only AFTER calling skills, begin implementation.

---

$PROMPT
EOF
