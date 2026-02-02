# PR Workflows

## Creating a PR
```bash
jj log
jj bookmark set feature/my-feature
jj git push --bookmark feature/my-feature
# Or auto-create bookmark:
jj git push -c @
```

## Responding to PR Review
```bash
jj edit <pr-rev>

# Option 1: Amend current change directly

# Option 2: Add fixup commit
jj new -m "address review: fix validation"
# ... make changes ...
jj squash  # Squash into parent

jj git push
```

## Stacked PRs
```bash
# Base feature
jj new main -m "feat: base layer"
jj bookmark set feature/base

# Enhancement on top
jj new -m "feat: enhancement layer"
jj bookmark set feature/enhancement

# Push all
jj git push --bookmark feature/base
jj git push --bookmark feature/enhancement

# When base is merged, rebase others
jj git fetch
jj rebase -s feature/enhancement -d main
```
