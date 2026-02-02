---
name: skill-creator
description: Creates, modifies, and validates skills following best practices. Use when creating new skills, modifying existing skills, or validating skill structure.
allowed-tools:
  - Bash
  - Read
  - Edit
  - Write
  - Glob
user-invocable: true
argument-hint: <mode> [skill-path]
---

# Skill Creator

## Contents

- [Overview](#overview)
- [When to Use](#when-to-use)
- [The Process](#the-process)
- [Tips](#tips)
- [Checklist](#checklist)

## Overview

Creates and maintains Claude Code skills following established best practices. Supports three modes: creating new skills, modifying existing skills, and validating skill structure.

## When to Use

- Creating a new skill from scratch
- Modifying or improving an existing skill
- Validating a skill before deployment
- Fixing validation errors or warnings

## The Process

### Mode A: Create New Skill

1. Create skill directory with SKILL.md, README.md, AGENTS.md
2. Edit SKILL.md:
   - Write description in third person ("Creates...", "Analyzes...")
   - Include "Use when" trigger conditions
   - Keep description 100-300 characters
3. Edit README.md with Japanese translation
4. Run Mode C to validate
5. Deploy the skill

### Mode B: Modify Existing Skill

1. Read all files in the skill directory
2. Check references/best-practices.md for guidelines
3. Apply modifications following best practices
4. Sync README.md with SKILL.md changes
5. Run Mode C to validate

### Mode C: Validate Skill

1. Perform static validation on skill files
2. If errors exist, fix them before proceeding
3. Read all skill files (SKILL.md, README.md, references/*)
4. Check against best practices:

**Description Validation**
- Written in third person ("Creates...", "Analyzes...")
- Contains "Use when" clause
- Length 100-300 characters (recommended)
- No forbidden characters (`<` `>`)

**Structure Validation**
- Has Overview, When to Use, The Process, Tips sections
- The Process contains numbered steps

**Size Validation**
- SKILL.md under 500 lines
- Under 5000 words total

**Reference Validation**
- references/ depth is 1 level only
- Files over 100 lines have table of contents

**Forbidden Files**
- No CHANGELOG.md, INSTALLATION_GUIDE.md, QUICK_REFERENCE.md

**Sync Validation**
- README.md matches SKILL.md content

5. Report issues with location and suggested fix
6. Auto-fix if requested

## Tips

- Description: third person + "Use when" + 100-300 chars
- Keep SKILL.md under 500 lines, under 5000 words
- Place important info at the top of files
- Add table of contents to files over 100 lines
- Keep references/ to 1 level depth only
- Always sync README.md with SKILL.md
- No first person ("I can help..." is forbidden)
- No time-dependent info ("As of 2024..." is forbidden)

## Checklist

- [ ] description: third person + "Use when" + 100-300 chars + no `<>`
- [ ] name: matches `^[a-z0-9][a-z0-9-]*[a-z0-9]$`, under 64 chars
- [ ] SKILL.md: under 500 lines, under 5000 words
- [ ] structure: Overview, When to Use, The Process, Tips
- [ ] references/: 1 level depth only
- [ ] 100+ line files: have table of contents
- [ ] forbidden files: none present
- [ ] README.md: synced with SKILL.md
- [ ] AGENTS.md: contains sync instructions
