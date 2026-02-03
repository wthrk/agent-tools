# Concurrent Execution Workflows

## Multiple Agent Setup
```bash
# Leader agent: Setup workspaces
jj workspace add ../agent-a --rev main -m "Feature A"
jj workspace add ../agent-b --rev main -m "Feature B"

# Agent A (in ../agent-a)
jj status
# ... work ...

# Agent B (in ../agent-b)
jj status
# ... work ...

# Leader: Merge results
jj workspace list
jj log -r 'working_copies()'

# Cleanup
jj workspace forget agent-a
jj workspace forget agent-b
```

## Recovery from File Mixing

If multiple agents worked in same directory without workspaces:

```bash
# 1. Check operation history
jj op log

# 2. Try to revert to clean state
jj undo
# Or:
jj op restore <last-good-op-id>

# 3. If salvage needed, separate contaminating files
jj diff                           # Identify which files don't belong
jj split --path <unwanted-file>   # Split out contaminating files
jj abandon                        # Abandon the split-off change

# 4. Last resort: abandon entire change
jj abandon <contaminated-change>
jj new main -m "Clean restart"
```

**Prevent recurrence:** Use workspaces for concurrent agents.

## When to Use Workspaces

**Required:**
- Multiple AI agents running in parallel
- CI/builds running alongside development
- File watchers modifying files

**Not needed:**
- Single developer/agent switching between tasks (use `jj edit`)
