---
name: managing-agents-md
description: Creates and manages AGENTS.md files following a widely used standard format for AI coding agents. Use when initializing a new project for AI agents, documenting project conventions, or validating existing AGENTS.md files.
allowed-tools:
  - Bash
  - Read
  - Edit
  - Write
  - Glob
  - Grep
user-invocable: true
argument-hint: [create|update|validate]
---

# Managing AGENTS.md

## Contents

- [Overview](#overview)
- [When to Use](#when-to-use)
- [The Process](#the-process)
  - [Mode A: View or Create](#mode-a-view-or-create)
  - [Mode B: Create](#mode-b-create)
  - [Mode C: Update Section](#mode-c-update-section)
  - [Mode D: Validate](#mode-d-validate)
- [Template](#template)
- [Rules](#rules)
- [Tips](#tips)

## Overview

Manages AGENTS.md files, the standard documentation format for AI coding agents. Supports viewing, creating, updating, and validating.

**Key facts:**
- Standard Markdown, no required fields
- Monorepos support nested AGENTS.md (closest file takes precedence)
- 32KiB size limit (Codex default)
- Used by Codex, Cursor, Copilot, and other AI agents

## When to Use

- Setting up a new project for AI agent collaboration
- Documenting project-specific conventions and commands
- Updating outdated project information
- Validating AGENTS.md structure and completeness

## The Process

### Mode A: View or Create

Invoked with no arguments: `/agents-md`

1. Check if AGENTS.md exists in project root
2. If exists: display contents with summary
3. If not exists: offer to create one

### Mode B: Create

Invoked with: `/agents-md create`

1. **Detect project information:**
   - Scan for package.json, Cargo.toml, pyproject.toml, go.mod, etc.
   - Analyze directory structure
   - Extract info from existing README.md
2. **Present detection results** to user for confirmation
3. **Ask questions** for missing information:
   - Project description (if not in README)
   - Test commands
   - Special conventions or boundaries
4. **Generate AGENTS.md** using template
5. **Show preview** and confirm before writing

### Mode C: Update Section

Invoked with: `/agents-md update <section>`

Valid sections: overview, stack, structure, commands, style, testing, boundaries, security

1. Read existing AGENTS.md
2. Locate the specified section
3. Apply updates (preserve other sections)
4. Show diff and confirm before writing

### Mode D: Validate

Invoked with: `/agents-md validate`

Checks:
1. **Structure:** Required sections present
2. **Line count:** Warn if > 150 lines
3. **Commands:** Code blocks exist for executable commands
4. **No secrets:** Warn if potential credentials detected
5. **Completeness:** Score based on 6 core sections

Output validation report with score and recommendations.

## Template

```markdown
# Agent Instructions

## Project Overview
[1-2 sentences: what this project does and its primary technology]

## Technology Stack
- Language: [language and version]
- Framework: [framework and version]
- Package Manager: [package manager]
- [Additional dependencies]

## Directory Structure
```
[tree format, key directories only]
```

## Development Commands
```bash
# Install dependencies
[install command]

# Run tests
[test command]

# Build
[build command]

# Start development server
[dev server command]
```

## Code Style Guidelines
- [Specific rule 1]
- [Specific rule 2]
- [Error handling conventions]

## Testing
- Framework: [test framework]
- Coverage: [coverage requirements]
- Run: `[test command]`

## Boundaries

### Always Do
- [Required practice 1]
- [Required practice 2]

### Never Do
- Do not commit secrets or credentials
- Do not modify [protected files/directories]
- [Other prohibitions]

## Security Considerations
- [Security note 1]
- [Environment variable handling]
```

## Rules

**FACTS:**
- AGENTS.md is placed at project root
- Can be nested in subdirectories (closest file takes precedence)
- Standard Markdown, no required fields
- Multiple agents (Codex, Cursor, Copilot, etc.) reference the same file

**RULES:**
- Always confirm before overwriting existing file
- Respect existing content, update only necessary sections
- Write commands in executable format within code blocks
- Target 150 lines or fewer
- Place commands early in the file (with flags)
- Prefer code examples over explanations

**6 Core Sections (prioritized):**
1. Commands - build, test, run commands
2. Testing - framework, coverage, special setup
3. Directory Structure - directory layout
4. Code Style - formatting, patterns, linters
5. Git Workflow - branch strategy, commit conventions
6. Boundaries - what not to touch

**WARNINGS:**
- Overwriting existing AGENTS.md may cause data loss
- Never include secrets or credentials
- Avoid duplicate information, link to external resources

## Tips

- **Detection priority:** package.json > Cargo.toml > pyproject.toml > go.mod
- **Common stack patterns:**
  - Node.js: look for `scripts` in package.json
  - Rust: look for `[[bin]]` and `[workspace]` in Cargo.toml
  - Python: look for `[tool.pytest]` or `[tool.poetry]`
- **Recovery from mistakes:** `jj undo` or `git checkout AGENTS.md`
- **Monorepo support:** Create nested AGENTS.md in subdirectories for package-specific instructions
