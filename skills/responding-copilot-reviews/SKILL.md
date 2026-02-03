---
name: responding-copilot-reviews
description: Responds to GitHub Copilot review comments on PRs. Use when addressing review comments or reviewing PRs.
allowed-tools:
  - Bash
  - Read
  - Edit
  - Write
  - Glob
  - Grep
  - Task
user-invocable: true
argument-hint: "[PR number]"
---

# Responding to Copilot Reviews

## Contents

- [Overview](#overview)
- [When to Use](#when-to-use)
- [The Process](#the-process)
  - [Step 1: Fetch Copilot Comments](#step-1-fetch-copilot-comments)
  - [Step 2: Spawn Subagents for Each Comment](#step-2-spawn-subagents-for-each-comment)
  - [Step 3: Collect Results and Make Decisions](#step-3-collect-results-and-make-decisions)
  - [Step 4: Fix in Separate Commits](#step-4-fix-in-separate-commits)
  - [Step 5: Reply to Each Review Comment](#step-5-reply-to-each-review-comment)
- [Subagent Task](#subagent-task)
- [Anti-patterns](#anti-patterns)
- [Tips](#tips)

## Overview

Structured workflow for responding to GitHub Copilot's automated code review comments on PRs. Key principle: **critical evaluation** - not all Copilot suggestions are valid.

## When to Use

- PR has Copilot review comments to address
- Systematically evaluate automated review feedback
- Respond to multiple review comments on a PR

## The Process

### Step 1: Fetch Copilot Comments

Retrieve review comments with pagination, filtering to Copilot only:

```bash
gh api --paginate repos/{owner}/{repo}/pulls/{pr}/comments \
  | jq '[.[] | select(.in_reply_to_id == null and .user.login == "Copilot")]'
```

**Filter criteria:**
- `in_reply_to_id == null` - Skip already-replied comments (avoid re-replying)

**Required fields from each comment:**
- `id` - Comment ID (needed for Step 5 replies)
- `path` - File path
- `line` / `original_line` - Line number (may be null for outdated)
- `diff_hunk` - Code context (fallback when line is null)
- `body` - Comment content

### Step 2: Spawn Subagents for Each Comment

For each comment, spawn a subagent in parallel using Task tool:

```
Task tool (subagent_type: "general-purpose", run_in_background: true)
prompt: |
  Evaluate this Copilot review comment:

  COMMENT_ID: {id}
  File: {path}
  Line: {line} (if null, use diff_hunk below)
  Diff Hunk: {diff_hunk}
  Comment: {body}

  1. Read the actual code at {path}
     - If line is valid: read around that line
     - If line is null: use diff_hunk to locate context
  2. Verify whether the suggestion is correct
  3. Use Codex to get a second opinion:
     codex exec "Is this suggestion valid? [comment]. Context: [code snippet]"
  4. Return verdict with COMMENT_ID preserved (required for reply)
```

**CRITICAL:**
- Launch all subagents in parallel (single message with multiple Task calls)
- Limit to 5-10 concurrent subagents to avoid rate limits
- Each subagent MUST return the COMMENT_ID in output

### Step 3: Collect Results and Make Decisions

Wait for all subagents to complete, then create decision table **with COMMENT_ID preserved**:

| COMMENT_ID | File | Subagent Verdict | Decision |
|------------|------|------------------|----------|
| 123456 | src/main.rs | ACCEPT - valid suggestion | ✅ Accept |
| 123457 | src/lib.rs | REJECT - false positive | ❌ Reject |
| 123458 | src/utils.rs | ACCEPT - improves readability | ✅ Accept |

**Error handling:** If a subagent fails or times out:
- Retry the evaluation manually for that comment
- Or mark as NEEDS_REVIEW and handle synchronously

### Step 4: Fix in Separate Commits

For accepted suggestions, create commits:

```bash
jj new -m "style: address review comment - [description]"
# Apply fix(es)
jj git push
```

**Commit strategy:**
- **Default:** One commit per fix (easier to revert, clearer history)
- **Exception:** Batch fixes of the same type in one commit
- **NEVER:** Mix fixes into original commits

### Step 5: Reply to Each Review Comment

Reply directly to the review comment (not as general PR comments):

**For accepted suggestions:**
```bash
gh api repos/{owner}/{repo}/pulls/{pr}/comments/{id}/replies \
  -f body="✅ Fixed in commit {hash}"
```

**For rejected suggestions:**
```bash
gh api repos/{owner}/{repo}/pulls/{pr}/comments/{id}/replies \
  -f body="❌ False positive. [reason]"
```

**RULE:** Always reply to review comments directly, not as issue comments.

## Subagent Task

Each subagent evaluates ONE comment and must:

1. **Read the code** - Fetch the file and locate the referenced line
   - If `line` is null or outdated, use `diff_hunk` to find context
   - Search for unique strings from `diff_hunk` in the file
2. **Understand context** - Read surrounding code to understand intent
3. **Verify suggestion** - Is the Copilot suggestion technically correct?
4. **Consult Codex** - Get external opinion with `codex exec`
5. **Return verdict** - ACCEPT or REJECT with reasoning and **COMMENT_ID preserved**

**Subagent output format:**
```
VERDICT: ACCEPT | REJECT
COMMENT_ID: {id}
FILE: {path}
REASONING: {explanation}
FIX_SUGGESTION: {if ACCEPT, describe the fix}
```

**Why subagents?**
- Parallel processing for faster review
- Each comment gets focused attention
- Independent Codex consultations prevent bias
- Clear accountability per comment

## Anti-patterns

| Anti-pattern | Why It's Bad | Correct Approach |
|--------------|--------------|------------------|
| Blindly accepting all suggestions | False positives waste time; wrong fixes harm code | Critically verify each |
| Replying as general comments | Breaks review thread; hard to track | Reply directly to review comments |
| Mixing fixes into original commits | Pollutes commit history; hard to revert | Separate commits per fix |
| Making decisions without discussion | May miss valid counterpoints | Discuss with Codex first |

## Tips

- **Never skip verification**: Step 2 and Step 3 are mandatory - critical verification is the core principle
- **Handle outdated comments**: When `line` is null, use `diff_hunk` or `original_line` to locate context
- **Document rejections well**: Future reviewers benefit from clear reasoning
- **Preserve COMMENT_ID throughout**: Without IDs, you cannot reply to the correct comment in Step 5
- **Use `gh api` over `gh pr comment`**: Direct API gives more control over comment threading
