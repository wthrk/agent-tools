---
name: reviewing
description: Performs dual-review with self-review and Codex review in parallel, then reconciles findings. Use when asked to review code, documents, or any content.
allowed-tools:
  - Read
  - Glob
  - Grep
  - Task
  - Bash(codex exec*)
user-invocable: true
argument-hint: "[target]"
---

# Reviewing

## Contents

- [Overview](#overview)
- [When to Use](#when-to-use)
- [The Process](#the-process)
  - [Step 1: Parallel Reviews](#step-1-parallel-reviews)
  - [Step 2: Reconciliation](#step-2-reconciliation)
  - [Step 3: Final Report](#step-3-final-report)
- [Tips](#tips)

## Overview

Dual-review process combining self-review and Codex review. Both reviews run in parallel, then findings are reconciled to form consensus.

## When to Use

- Code review requests
- Document review requests
- PR review requests
- Any content that needs objective evaluation

## The Process

### Step 1: Parallel Reviews

Execute BOTH reviews in parallel (single message, two tool calls):

**Self-Review Agent (Task tool):**
```
Task tool (subagent_type: "general-purpose")
prompt: |
  Review the following code/content and identify: bugs, logic errors, security concerns, style violations, and improvements for readability/performance/maintainability. Be specific with locations and reasoning. Target: [target]
```

**Codex Review (Bash tool):**
```bash
codex exec "Review the following code/content. Identify bugs, logic errors, security concerns, style violations, and improvements. Be specific with file paths and line numbers. Target: [target]"
```

IMPORTANT: Call both tools (Task + Bash) in a SINGLE message to achieve true parallel execution.

### Step 2: Reconciliation

Compare both review results:

1. **Agreements**: Issues both reviewers found (high confidence)
2. **Self-only findings**: Issues only self-review found (discuss validity)
3. **Codex-only findings**: Issues only Codex found (discuss validity)

For disagreements:
- Evaluate evidence and reasoning from both sides
- Consider context and project conventions
- Form consensus or note as "disputed"

### Step 3: Final Report

Present unified findings in this format:

```
## Review Summary

### Agreed Issues (High Confidence)
- [Location]: [Issue] - [Recommendation]

### Additional Findings
- [Location]: [Issue] - [Source: Self/Codex] - [Recommendation]

### Disputed Points (if any)
- [Location]: [Self opinion] vs [Codex opinion]
```

## Tips

- Start both reviews immediately in parallel for efficiency
- Be specific about file paths and line numbers
- Prioritize issues by severity: critical > major > minor
- Include positive observations, not just problems
- For large targets, break into logical chunks
