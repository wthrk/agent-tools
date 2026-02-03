# jj Troubleshooting Guide

## Contents

- [Common Errors and Solutions](#common-errors-and-solutions)
- [Recovery Techniques](#recovery-techniques)
- [Agent-Specific Issues](#agent-specific-issues)
- [Useful Diagnostic Commands](#useful-diagnostic-commands)

## Common Errors and Solutions

### "bookmark not found"

**Error:**
```
Error: No bookmark exists for the revision
```

**Solution:**
```bash
# Set bookmark before pushing
jj bookmark set <name>
jj git push --bookmark <name>

# Or use -c to auto-create
jj git push -c @
```

---

### "conflicting changes"

**Error:**
```
Error: Conflicting changes in working copy
```

**Solution:**
```bash
# View conflicts
jj status
jj diff

# Resolve by editing files, then:
jj squash
# Or abandon and retry:
jj undo
```

---

### "operation blocked" (interactive mode)

**Error:**
Agent hangs waiting for interactive input.

**Cause:**
Commands like `jj split` default to interactive mode.

**Solution:**
```bash
# Use non-interactive flags
jj split --path <file>

# Or specify exact paths to split
jj split --path src/main.rs --path src/lib.rs
```

---

### "nothing to push"

**Error:**
```
Nothing changed (no bookmarks to push).
```

**Solution:**
```bash
# Check bookmark status
jj bookmark list

# Set bookmark on current change
jj bookmark set <name>

# Then push
jj git push --bookmark <name>
```

---

### "rebase conflict"

**Error:**
```
Rebasing X onto Y resulted in conflicts
```

**Solution:**
```bash
# View operation log
jj op log

# Option 1: Undo rebase
jj undo

# Option 2: Resolve conflicts manually
jj status
# Edit conflicting files
jj squash
```

---

### "change not found"

**Error:**
```
Error: Revision "xyz" doesn't exist
```

**Solution:**
```bash
# Find correct revision
jj log --limit 20

# Use change ID (short form)
jj edit abc  # First few letters usually enough
```

---

### "cannot edit immutable commit"

**Error:**
```
Error: Cannot edit immutable commits
```

**Cause:**
Trying to edit commits that are protected (e.g., already pushed to remote).

**Solution:**
```bash
# Create new change on top instead
jj new <immutable-rev> -m "changes on top"

# Or check immutable settings
# In jj config, immutable_heads() defines protected commits
```

---

### "divergent bookmarks"

**Error:**
```
Bookmark 'main' points to multiple commits
```

**Solution:**
```bash
# View divergent state
jj bookmark list

# Force update to specific revision
jj bookmark set main -r <rev> --allow-backwards

# Or delete and recreate
jj bookmark delete main
jj bookmark set main
```

---

## Recovery Techniques

### Undo Last Operation
```bash
jj undo
```

### View All Operations
```bash
jj op log
```

### Restore to Specific Point
```bash
jj op log
# Find operation ID
jj op restore <op-id>
```

### Recover Abandoned Change
```bash
# Immediately after abandon
jj undo

# Or find in op log and restore
jj op log
jj op restore <before-abandon-op-id>
```

### View Evolution of a Change
```bash
jj evolog -r <rev>
```

---

## Agent-Specific Issues

### Split Command Blocking

**Problem:** `jj split` blocks waiting for interactive input.

**Prevention:**
```bash
# Always use --path for non-interactive
jj split --path <file>
```

### Unintended Rebase Scope

**Problem:** `jj rebase` affects more changes than intended.

**Prevention:**
```bash
# Always specify source explicitly
jj rebase -s <rev> -d main

# Check with log first
jj log -r 'main..@'
```

### Missing Bookmark on Push

**Problem:** Push fails silently or warns about no bookmarks.

**Prevention:**
```bash
# Always set bookmark before push workflow
jj bookmark set <name>
jj git push --bookmark <name>
```

---

## Useful Diagnostic Commands

```bash
# Current state
jj status
jj log

# Operation history
jj op log

# Evolution history
jj evolog

# Compare revisions
jj interdiff --from <rev1> --to <rev2>

# Workspace status
jj workspace list
```
