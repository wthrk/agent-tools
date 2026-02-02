---
name: jj
description: Manages Jujutsu (jj) repositories with change-centric workflows. Use when working with jj repos, creating changes, rebasing, or pushing.
allowed-tools: Bash, Read, Glob, Grep
user-invocable: true
argument-hint: "[new|describe|status|log|diff|edit|split|squash|abandon|restore|bookmark|rebase|push|undo]"
---

# jj - Jujutsu Version Control Skill

## Contents

- [CRITICAL: Execution Policy](#critical-execution-policy)
- [Philosophy: Parallel Development by Default](#philosophy-parallel-development-by-default)
- [Parallelism Model](#parallelism-model)
- [Quick Reference](#quick-reference)
- [Commands](#commands)
  - [new](#jj-new---create-new-change)
  - [describe](#jj-describe---set-change-message)
  - [status](#jj-status---file-level-status)
  - [log](#jj-log---change-history)
  - [diff](#jj-diff---show-changes)
  - [edit](#jj-edit---switch-task--navigate)
  - [split](#jj-split---split-change)
  - [squash](#jj-squash---merge-into-parent)
  - [abandon](#jj-abandon---discard-change)
  - [restore](#jj-restore---restore-files)
  - [bookmark](#jj-bookmark---manage-bookmarks)
  - [rebase](#jj-rebase---rebase-to-main)
  - [push](#jj-push---push-to-remote)
  - [undo](#jj-undo---undo-operation)
- [Advanced Features](#advanced-features-guidance-only)
- [Agent Protocol](#agent-protocol)
- [Core Concepts](#core-concepts)
- [Common Workflows](#common-workflows)
- [Error Recovery](#error-recovery)
- [Colocated Mode](#colocated-mode)

## CRITICAL: Execution Policy

**NEVER execute Bash commands when invoked without specific arguments.**

When `/jj <subcommand>` is invoked without arguments:
1. **DO NOT call Bash** - Do not attempt to run any jj command
2. **Show the documentation section** for that subcommand from this file
3. **Include code block examples** from this file
4. **Ask what the user wants to do**

When invoked with specific arguments (e.g., `/jj new main -m "task"`):
- Execute the command and report results

**Read-only commands** (`/jj status`, `/jj log`, `/jj diff`):
- May execute immediately to show current state

## Philosophy: Parallel Development by Default

All work is treated as "one of many parallel tasks" (single task = N=1 special case).

- Task = independent change (`jj new main -m "task"`)
- Switch between tasks anytime (`jj edit`)
- Everything is undoable via operation log
- jj solves thought/history parallelism (execution parallelism only via workspaces)

## Parallelism Model

| Scenario | Method | Directory |
|----------|--------|-----------|
| Single agent switching tasks | `jj edit` | Single |
| Multiple agents simultaneously | `jj workspace` | Separate per agent |

**Why workspace for concurrent execution:** Single working copy = single file system state. Multiple writers = race condition requiring recovery.

**Definition:** "Simultaneous" = any concurrent file system writer (agents, CI, watchers, builds).

## Quick Reference

| Category | Commands |
|----------|----------|
| Core | new, describe, status, log, diff |
| Navigation | edit (also `@-` for prev, `@+` for next) |
| Organization | split, squash, abandon, restore |
| Sync | bookmark, rebase, push, undo |

---

## Commands

### /jj new - Create New Change

Creates a new empty change. The fundamental jj operation.

```bash
# Create empty change on current
jj new

# Create with message
jj new -m "message"

# Create new task from main (RECOMMENDED for starting work)
jj new main -m "feat: implement feature X"

# Create change BEFORE current (insert)
jj new -B @ -m "insert before current"
```

**Use cases:**
- After completing work, create next step
- Start new independent task from main
- Insert change in the middle of stack

---

### /jj describe - Set Change Message

Sets or updates description for the current or specified change.

```bash
# Set message for current change
jj describe -m "feat: add user authentication"

# Edit past change's message
jj describe -r <rev> -m "updated message"
```

---

### /jj status - File-Level Status

Shows file-level working copy status. Fast and concise.

```bash
jj status
```

**Output shows:**
- Modified files
- Added/deleted files
- Current change ID

**Note:** Use `status` for quick "what files changed?" check. Use `log` for change-level overview.

---

### /jj log - Change History

Shows change history and change-level status.

```bash
# Show recent history
jj log

# Limit output
jj log --limit 10

# Show all tasks (useful for parallel development)
jj log -r 'all()'

# Show task tree from main
jj log -r 'main..@'
```

---

### /jj diff - Show Changes

Shows diff of current or specified change.

```bash
# Current change diff
jj diff

# Specific change
jj diff -r <rev>

# Specific file
jj diff <path>

# Summary only
jj diff --stat
```

---

### /jj edit - Switch Task / Navigate

Switches working copy to a different change. Essential for parallel development.

```bash
jj edit <rev>   # Switch to specific revision
jj edit @-      # Navigate to parent
jj edit @+      # Navigate to child
```

---

### /jj split - Split Change

**WARNING: INTERACTIVE BY DEFAULT - WILL BLOCK!**

Splits a change into multiple smaller changes.

**CRITICAL for agents:** Default `jj split` is interactive and will block indefinitely.
Always use `--path` flag for non-interactive operation.

```bash
# WARNING: Interactive - BLOCKS the agent!
jj split

# RECOMMENDED: Non-interactive with --path flag
jj split --path <file1> --path <file2>

# Split specific revision (also use --path)
jj split -r <rev> --path <file>
```

**Agent Protocol:** MUST use `--path` flag. Never run bare `jj split`.

---

### /jj squash - Merge into Parent

Merges current working copy into parent change.

```bash
# Squash all into parent
jj squash

# Interactive selection
jj squash -i

# Squash specific files
jj squash <paths>
```

**Use case:** Merge WIP into completed parent change.

---

### /jj abandon - Discard Change

Intentionally discards a change. Important for cleanup.

```bash
# Abandon current change
jj abandon

# Abandon specific change
jj abandon <rev>

# Abandon multiple
jj abandon <rev1> <rev2>
```

**Note:** Abandoned changes can be recovered via `jj undo`.

---

### /jj restore - Restore Files

Restores files from a specific revision. Equivalent to `git checkout -- <file>`.

```bash
# Restore file from parent
jj restore --from @- <path>

# Restore all files from revision
jj restore --from <rev>

# Restore from change ID
jj restore --from abc123 <path>
```

---

### /jj bookmark - Manage Bookmarks

Bookmarks are named references needed for pushing to remotes.

```bash
# Set bookmark on current change
jj bookmark set <name>

# Set on specific revision
jj bookmark set <name> -r <rev>

# List bookmarks
jj bookmark list

# Delete bookmark
jj bookmark delete <name>

# Track remote bookmark
jj bookmark track <name>@<remote>
```

**Note:** jj push requires bookmarks. Set before pushing.

---

### /jj rebase - Rebase to Main

Rebases changes onto updated main branch.

**WARNING:** Without explicit scope, may rebase unintended changes.

```bash
# Step 1: Detect main branch
jj bookmark list | grep -E '(main|master)'

# Step 2: Fetch updates
jj git fetch

# Step 3: Rebase current change only
jj rebase -d main

# Rebase specific change
jj rebase -s <rev> -d main

# Rebase all roots (parallel tasks)
jj rebase -s 'all:roots(main..@)' -d main
```

**Scope Policy:**
- Default: current change only
- Multiple roots: warn and require explicit flag
- Always verify with `jj log` after rebase

---

### /jj push - Push to Remote

**IMPORTANT: Bookmark is REQUIRED for pushing.**

Pushes bookmarked changes to remote. Without a bookmark, push will fail.

```bash
# Push all bookmarks
jj git push

# Push specific bookmark
jj git push --bookmark <name>

# Push current change (auto-creates bookmark)
jj git push -c @

# Push all tracked bookmarks
jj git push --tracked
```

**Workflow:**
```bash
# 1. First set a bookmark
jj bookmark set feature-x

# 2. Then push
jj git push --bookmark feature-x
```

**Note:** If no bookmark exists, suggest creating one before push.

---

### /jj undo - Undo Operation

Undoes the last operation using operation log. Safety net for all jj operations.

```bash
# Show operation history
jj op log

# Undo last operation
jj undo

# Undo specific operation
jj op restore <op-id>
```

**Use cases:**
- Recover from accidental abandon
- Revert failed rebase
- Undo experimental operations

---

## Advanced Features (Guidance Only)

### absorb - Auto-distribute Changes
`jj absorb` - Distributes working copy changes to appropriate ancestors automatically.

### evolog - Evolution History
`jj evolog [-r <rev>]` - Shows evolution history of a change.

### interdiff - Compare Revisions
`jj interdiff --from <rev1> --to <rev2>` - Compares two revisions (useful for PR review).

### workspace - Concurrent Execution

Creates separate working directories for multiple simultaneous processes.

```bash
# Setup for multiple agents
jj workspace add ../agent-a --rev main -m "Agent A task"
jj workspace add ../agent-b --rev main -m "Agent B task"

# Each agent works in its own directory
cd ../agent-a && claude "..."
cd ../agent-b && claude "..."

# Cleanup
jj workspace forget agent-a
```

**When required:**
- Multiple AI agents running in parallel
- Any scenario with simultaneous file system access

**When NOT needed:**
- Single developer/agent switching between tasks (use `jj edit`)

---

## Agent Protocol

### FACTS (from jj documentation)
- Operation log records all operations
- `jj undo` reverts last operation
- Working copy always belongs to one change
- `jj split` is interactive by default

### RULES (workflow rules from jj design)
- Always declare change boundary before editing (`jj new`/`jj edit`)
- Maintain 1 task = 1 change
- Use operation log as safety net
- Use `--path` for non-interactive `jj split`
- Concurrent agents require separate workspaces (single directory = file conflicts)

### WARNINGS (for agents)
- `jj split` without `--path` will BLOCK
- `jj squash -i` (interactive) will BLOCK
- `jj rebase` without scope may rebase unintended changes
- Push without bookmark will FAIL
- Multiple agents in single directory will cause file mixing → use `jj workspace` or recover with `jj abandon` + `jj new`
- CI/file watcher running in background may increase conflict risk

---

## Core Concepts

- **Change** (mutable, `abc123`) vs **Commit** (immutable, hash)
- `@` = current, `@-` = parent, `@+` = child, `@--` = grandparent
- **Operation log**: All commands recorded, `jj undo` reverts, `jj op restore <id>` restores
- **Bookmarks**: Named pointers required for pushing

---

## Common Workflows

**Quick patterns:**
- New work: `jj git fetch && jj new main -m "feat: X" && jj bookmark set X && jj git push -b X`
- Switch task: `jj edit <rev>`
- Update from main: `jj git fetch && jj rebase -d main`

**Detailed workflows:** [workflow-basics.md](references/workflow-basics.md) | [workflow-pr.md](references/workflow-pr.md) | [workflow-concurrent.md](references/workflow-concurrent.md)

---

## Error Recovery

See [references/troubleshooting.md](references/troubleshooting.md) for common errors and solutions.

Quick fixes:
- **Undo any mistake:** `jj undo`
- **Restore to known good state:** `jj op restore <id>`
- **Recover abandoned change:** `jj undo` immediately after abandon

---

## Colocated Mode

Recommended for Git compatibility:

```bash
jj git init --colocate      # New repo
jj git init --colocate .    # Convert existing
```

Benefits: Git tools + IDE integration preserved.
