# Claude Code Skill Best Practices

## Table of Contents

- [1. Description](#1-description)
- [2. Frontmatter](#2-frontmatter)
- [3. SKILL.md Structure](#3-skillmd-structure)
- [4. Claude Reading Behavior](#4-claude-reading-behavior)
- [5. Progressive Disclosure](#5-progressive-disclosure)
- [6. Directory Structure](#6-directory-structure)
- [7. Script Design](#7-script-design)
- [8. Static Analysis](#8-static-analysis)
- [9. Content Principles](#9-content-principles)
- [10. Multilingual Support](#10-multilingual-support)
- [11. Checklist](#11-checklist)

---

## 1. Description

### Format
```
[Feature description]. Use when [trigger conditions].
```

### Constraints
| Item | Limit |
|------|-------|
| Maximum | 1024 characters |
| Recommended | 100-300 characters |
| Forbidden | `<` `>` |

### Principles
- Third person ("Creates...", "Analyzes...")
- Include keywords
- Specify trigger conditions
- Do not include workflow details

### Good Examples
```
Scans Algorand smart contracts for 11 common vulnerabilities. Use when auditing Algorand projects.
```

### Bad Examples
```
For async testing                    # Too vague
I can help you...                    # First person forbidden
```

### Note
The current format with feature description and trigger conditions is sufficient. Reconsider if issues arise.

---

## 2. Frontmatter

### Required Fields
```yaml
---
name: skill-name
description: ...
---
```

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

### Name Validation
- Regex: `^[a-z0-9][a-z0-9-]*[a-z0-9]$` or `^[a-z0-9]$`
- Maximum 64 characters
- No leading/trailing/consecutive hyphens

---

## 3. SKILL.md Structure

### Size Limits
| Item | Recommended | Maximum |
|------|-------------|---------|
| Lines | < 500 | - |
| Words | < 5,000 | 10,000 |

If exceeding 10,000 words, split by domain.

### Recommended Structure (Workflow Type)
```markdown
# Title
## Overview
## When to Use
## The Process
## Tips
```

### Alternative Structure (Reference Type)
```markdown
# Title
## Overview
## Quick Reference
## How to Use
```

---

## 4. Claude Reading Behavior

### Observed Tendencies
- Large files tend to be previewed partially
- Nested references tend to be read incompletely

### Countermeasures
| Issue | Solution |
|-------|----------|
| Partial reading | Files > 100 lines: table of contents at top |
| Nested references | 1 level depth only |
| Missing important info | Place at file top |

---

## 5. Progressive Disclosure

| Level | Content | Load Condition |
|-------|---------|----------------|
| 1 | Metadata | Always |
| 2 | SKILL.md body | On trigger |
| 3 | references/scripts/assets | On demand |

---

## 6. Directory Structure

```
skill-name/
├── SKILL.md                # Required
├── README.md               # Japanese (for multilingual)
├── AGENTS.md               # Sync instructions
├── references/
├── scripts/
└── assets/
```

### Not Recommended
- INSTALLATION_GUIDE.md
- QUICK_REFERENCE.md
- CHANGELOG.md (acceptable if multiple versions are maintained)

---

## 7. Script Design

### Prerequisites
Trusted in-skill scripts: check usage with --help before execution.

### Design Principles
- Single responsibility
- Explicit error handling
- JSON/markdown output
- No sensitive info in logs
- Document dependencies in README

---

## 8. Static Analysis

Perform static validation on your skill directory.

### Error Checks (Exit Code 1)
- SKILL.md existence
- Frontmatter format (`---` delimiters)
- YAML parsing
- Required fields (name, description)
- Name format and length
- Description forbidden characters and length
- Disallowed keys

### Warning Checks (Exit Code 2)
- Line count > 500
- Word count > 5000
- Forbidden files exist
- Reference depth > 1
- Missing table of contents in 100+ line files

---

## 9. Content Principles

### Core
1. Conciseness (Claude is already smart)
2. Consistent terminology
3. Concrete examples

### Avoid
- Time-dependent info ("As of 2024...")
- Multiple option presentations
- Windows paths
- First person ("I can help...")

---

## 10. Multilingual Support

### Structure
- SKILL.md: English
- README.md: Japanese (translation of SKILL.md)
- AGENTS.md: Sync instructions

### AGENTS.md Content
```markdown
README.md is the Japanese explanation of this skill.
When updating SKILL.md or any related files, also update README.md to keep them in sync.
```

---

## 11. Checklist

### Core
- [ ] description: third person + "Use when" + 100-300 chars
- [ ] name: regex match, under 64 chars
- [ ] SKILL.md: under 500 lines

### Structure
- [ ] references/: 1 level depth
- [ ] 100+ lines: table of contents
- [ ] README.md: synced with SKILL.md

### Multilingual
- [ ] SKILL.md: English
- [ ] README.md: Japanese
- [ ] AGENTS.md: sync instructions
