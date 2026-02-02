# Basic jj Workflows

## Starting New Work
```bash
jj git fetch
jj new main -m "feat: new feature"
# ... work ...
jj bookmark set feature-x
jj git push --bookmark feature-x
```

## Switching Between Tasks
```bash
jj log                    # See all tasks
jj edit <task-rev>        # Switch to task
# ... work ...
jj edit <other-task>      # Switch to another
```

## Updating from Main
```bash
jj git fetch
jj rebase -d main
# Or for all parallel tasks:
jj rebase -s 'all:roots(main..@)' -d main
```

## Stack Management
```bash
# Create stacked changes
jj new main -m "base feature"
jj new -m "enhancement on top"
jj new -m "final touches"

# Navigate stack
jj edit @-    # Go to parent
jj edit @+    # Go to child

# Squash WIP into parent
jj squash
```

## Organization

### Splitting Large Changes
```bash
jj diff
jj split --path src/feature.rs --path tests/feature_test.rs
# Specified files → FIRST change, remaining → SECOND change
```

### Squashing WIP
```bash
jj squash  # Merge current into parent
```

### Restoring Files
```bash
jj restore --from @- path/to/file.rs
```
