---
name: creating-skills
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

# Creating Skills

## Contents

- [Overview](#overview)
- [When to Use](#when-to-use)
- [The Process](#the-process)
  - [Mode A: Create New Skill](#mode-a-create-new-skill)
  - [Mode B: Modify Existing Skill](#mode-b-modify-existing-skill)
  - [Mode C: Validate Skill](#mode-c-validate-skill)
- [Rules](#rules)
  - [name](#name)
  - [description](#description)
  - [Optional Fields](#optional-fields)
  - [Structure](#structure)
  - [Progressive Disclosure](#progressive-disclosure)
  - [Directory Structure](#directory-structure)
  - [Script Design](#script-design)
  - [Content Principles](#content-principles)
  - [Multilingual](#multilingual)
  - [Static Analysis](#static-analysis)
- [Tips](#tips)

## Overview

Creates and maintains Claude Code skills. Supports three modes: create, modify, validate.

## When to Use

- Creating a new skill from scratch
- Modifying or improving an existing skill
- Validating a skill before deployment
- Fixing validation errors or warnings

## The Process

### Mode A: Create New Skill

1. Create skill directory with SKILL.md, README.md, AGENTS.md
2. Edit SKILL.md following Rules below
3. Edit README.md with Japanese translation
4. Run `agent-tools skill validate <path>` to check
5. Fix any errors or warnings

### Mode B: Modify Existing Skill

1. Read all files in the skill directory
2. Apply modifications following Rules below
3. Sync README.md with SKILL.md changes
4. Run `agent-tools skill validate <path>` to check

### Mode C: Validate Skill

1. Run `agent-tools skill validate <path>`
2. Fix errors before proceeding
3. Check against Rules below
4. Verify README.md matches SKILL.md content
5. Report issues with location and suggested fix

## Rules

### name

- Gerund form: `processing-pdfs`, `analyzing-data`
- Alternatives: `pdf-processing`, `process-pdfs`
- Avoid: `helper`, `utils`, `documents`
- Regex: `^[a-z0-9][a-z0-9-]*[a-z0-9]$`, max 64 chars

### description

Format: `[Feature description]. Use when [trigger conditions].`

- Third person: "Creates...", "Analyzes..."
- 100-300 chars recommended, max 1024
- No `<` `>`

Good: `Scans Algorand smart contracts for 11 common vulnerabilities. Use when auditing Algorand projects.`

Bad: `For async testing` (too vague), `I can help you...` (first person)

### Optional Fields

```yaml
license: MIT
allowed-tools: Read, Edit
metadata:
  author: name
  version: "1.0.0"
user-invocable: true
disable-model-invocation: false
argument-hint: <arg>
```

### Structure

Required sections: Overview, When to Use, The Process, Tips (additional sections like Contents, Rules allowed)

| Item | Recommended | Maximum |
|------|-------------|---------|
| Lines | < 500 | - |
| Words | < 5,000 | 10,000 |

Files over 100 lines need table of contents at top (include subsections).

### Progressive Disclosure

| Level | Content | Load Condition |
|-------|---------|----------------|
| 1 | Metadata | Always |
| 2 | SKILL.md body | On trigger |
| 3 | references/scripts/assets | On demand (not guaranteed) |

**Design Principle:** Level 3 may not be read.
- Rules that must be applied → Level 2 (SKILL.md body)
- Brief examples allowed in Level 2; extensive examples → Level 3 (references/)
- Skill must work correctly even if Level 3 is never read

### Directory Structure

```
skill-name/
├── SKILL.md                # Required
├── README.md               # Japanese (for multilingual)
├── AGENTS.md               # Sync instructions
├── references/             # Optional, 1 level depth only
├── scripts/
└── assets/
```

Not recommended: INSTALLATION_GUIDE.md, QUICK_REFERENCE.md, CHANGELOG.md

### Script Design

- Single responsibility
- Explicit error handling
- JSON/markdown output
- No sensitive info in logs
- Check usage with --help before execution

### Content Principles

Core:
1. Conciseness (Claude is already smart)
2. Consistent terminology
3. Concrete examples

Avoid:
- Time-dependent info ("As of 2024...")
- Long option lists (brief alternatives OK)
- Windows paths
- First person ("I can help...")

### Multilingual

- SKILL.md: English
- README.md: Japanese translation
- AGENTS.md: sync instructions

### Static Analysis

Error checks (exit code 1):
- SKILL.md existence, frontmatter format, YAML parsing
- Required fields (name, description)
- Name format/length, description forbidden chars/length

Warning checks (exit code 2):
- Line count > 500, word count > 5000
- Forbidden files, reference depth > 1
- Missing table of contents in 100+ line files

## Tips

- Common mistakes: first person in description, missing "Use when", vague names
- Place important info at file top (Claude may preview partially)
- Keep references/ to 1 level depth only (nested references read incompletely)
