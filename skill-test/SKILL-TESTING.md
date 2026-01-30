# Claude Code Skill Test

## Overview

Automatically verify that skills work correctly:

- **Invocation Test**: Is the Skill tool invoked? (reproducibility)
- **Behavior Validation**: Does the output meet assertions? (quality)

### Terminology

| Term | Definition |
|------|------------|
| **Skill Directory** | Directory containing SKILL.md |
| **Assertions** | Conditions the output must satisfy. Failure = Fail |
| **Golden Assertions** | Quality checks. Failure is recorded but doesn't fail the test |

---

## Directory Structure

```
my-skill/                           # Skill directory
  SKILL.md                          # Required (identifies as skill directory)
  skill-test.config.yaml            # Optional (per-skill configuration)
  skill-tests/
    test-basic.yaml
    test-advanced.yaml
    shared/                         # Shared assertions (within skill-tests)
      common-assertions.yaml
```

---

## CLI Usage

```bash
# Test current directory as skill directory
skill-test

# Specify skill directory
skill-test ./my-skill

# Multiple skill directories
skill-test ./skills/a ./skills/b

# With glob patterns (shell expansion)
skill-test ./skills/*

# Options
skill-test --filter "test-*"        # Filter by test ID
skill-test --format json            # Output format
skill-test --verbose                # Detailed output
```

### CLI Options

| Option | Default | Description |
|--------|---------|-------------|
| `[SKILL_DIR...]` | `.` | Skill directories to test |
| `--iterations` | config/10 | Iterations per test |
| `--hook` | config/simple | Hook strategy: `none`, `simple`, `forced`, `custom` |
| `--hook-path` | - | Custom hook script path (required with `--hook=custom`) |
| `--model` | config/sonnet | Model to use |
| `--timeout` | config/60000 | Timeout per iteration (ms) |
| `--threshold` | config/80 | Pass rate threshold (%) |
| `--strict` | false | Error on missing files |
| `--format` | `table` | Output: `table`, `json` |
| `--filter` | - | Filter test IDs (substring match) |
| `-v, --verbose` | false | Detailed output |
| `--no-color` | false | Disable colored output |
| `-p, --parallel` | CPU count | Parallel test executions (0 = sequential) |
| `--no-error-log` | false | Disable error log file output |

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (all skills passed) |
| 1 | Threshold not met |
| 2 | Configuration error |
| 3 | Execution error |

---

## Configuration: `skill-test.config.yaml`

```yaml
# All optional (defaults shown)
model: claude-sonnet-4-20250514
timeout: 60000
iterations: 10
threshold: 80  # >= 80% of iterations must pass

# Hook configuration
hook: simple  # none | simple | forced | custom
hook-path: ./path/to/hook.sh  # Required only when hook: custom

# Test file patterns (relative to skill directory)
test-patterns:
  - "skill-tests/**/test-*.yaml"
  - "skill-tests/**/test-*.yml"
  - "skill-tests/**/*.spec.yaml"
  - "skill-tests/**/*.spec.yml"

exclude-patterns:
  - node_modules/

# Behavior for missing files
strict: false  # true: ERROR (exit 1), false: WARN (stderr, continue)
```

---

## Test Case Format

Two formats are supported: **List format** (simple) and **Scenarios format** (advanced).

### List Format (Simple)

Test files as a YAML list of test cases:

```yaml
- id: test-001
  prompt: "Do something"
  iterations: 5  # Optional: override config
  assertions:
    - id: check-output
      type: contains
      pattern: "expected"
      expect: present
    - id: tool-check
      type: tool_called
      pattern: "Read|Write"  # Tool name regex
      expect: present
  golden_assertions:  # Quality tracking (doesn't affect pass/fail)
    - id: best-practice
      type: regex
      pattern: "^//"
      expect: present

- id: test-002
  prompt: "Another test"
  assertions:
    - file: ./shared/common-assertions.yaml  # External file reference
    - id: inline-check
      type: exec
      command: node
      language: javascript
      expect: exit_code:0
```

### Scenarios Format (Advanced)

Supports named assertions with reuse across scenarios:

```yaml
desc: "Search functionality tests"

# Named assertions (reusable across scenarios)
assertions:
  has-numbered-list:
    desc: "Contains numbered list"
    type: regex
    pattern: "\\d+\\."
    expect: present

  has-score:
    desc: "Shows score in X/100 format"
    type: regex
    pattern: "\\d+/100"
    expect: present

  has-source-url:
    type: regex
    pattern: "https?://"
    expect: present

# Test scenarios (key = scenario ID)
scenarios:
  search-basic:
    desc: "Basic search test"
    prompt: "Search for Claude Code skills"
    assertions:
      - has-numbered-list      # Name reference
      - has-score              # Name reference
      - has-source-url         # Name reference

  search-with-inline:
    desc: "Search with inline assertion"
    prompt: "Find testing skills"
    iterations: 5
    assertions:
      - has-numbered-list      # Name reference
      - id: inline-check       # Inline assertion
        type: contains
        pattern: "test"
        expect: present
    golden_assertions:
      - id: quality-check
        type: llm_eval
        pattern: |
          Does the output contain relevant search results?
          Answer YES or NO.

          Output:
          {{output}}
        expect: pass
```

**Scenarios format benefits:**
- Reusable named assertions reduce duplication
- Scenario ID comes from the key (no `id` field needed)
- `desc` field for human-readable test names
- Mix name references with inline assertions

---

## External Assertion Files

Reference external assertion files with `file:`:

```yaml
assertions:
  - file: ./common.yaml           # Single file
  - file:                         # Multiple files (merged in order)
      - ./base.yaml
      - ./strict.yaml
  - id: inline                    # Inline can be mixed
    type: contains
    pattern: "test"
    expect: present
```

### File Reference Rules

| Item | Behavior |
|------|----------|
| Merge order | File references processed first, then inline assertions |
| Duplicate ID | ERROR at load time |
| Circular reference | ERROR (detected via depth-first search) |
| Base path | Relative to the YAML file containing the reference |
| Outside skill-tests/ | ERROR (security boundary) |

---

## Configuration Priority

```
per-test > CLI flags > skill config > defaults
(more specific settings take precedence)
```

Example:
- defaults: `iterations=10`
- skill config: `iterations=5`
- CLI: `--iterations 3`
- test-001: `iterations: 1`

→ test-001 uses `iterations=1`, test-002 uses `iterations=3`

---

## Assertion Types

### regex

Regular expression pattern match.

```yaml
- id: "uses-state"
  type: regex
  pattern: "\\$state\\s*\\("
  expect: present    # present | absent
```

### contains

Simple string containment check.

```yaml
- id: "has-console-log"
  type: contains
  pattern: "console.log"
  expect: present
```

### line_count

Check output line count within range.

```yaml
- id: "reasonable-length"
  type: line_count
  min: 5             # Optional
  max: 100           # Optional (at least one required)
```

### exec

Extract code blocks and execute them.

```yaml
- id: "valid-javascript"
  type: exec
  command: "node"
  language: "javascript"    # Code block language (optional)
  timeout_ms: 5000          # Timeout (default: 10000)
  expect: "exit_code:0"     # Or output_contains
```

**expect formats:**
- `"exit_code:0"`: Exit code 0 = success
- `output_contains: "expected text"`: stdout contains string

### llm_eval

Semantic evaluation using LLM (Claude Haiku).

```yaml
- id: "follows-best-practices"
  type: llm_eval
  pattern: |
    Evaluate if the following code follows best practices.
    Answer YES or NO.

    Code:
    {{output}}
  expect: pass       # pass | fail
  timeout_ms: 60000  # Default: 60000
```

> **Note:** Due to non-determinism, recommended for golden_assertions only.

### tool_called

Check if specific tools were called during execution.

```yaml
- id: "uses-skill-tool"
  type: tool_called
  pattern: "Skill"           # Regex pattern for tool name
  expect: present

- id: "no-mcp-tools"
  type: tool_called
  pattern: "mcp__.*"
  expect: absent
```

---

## Hook Strategies

| Strategy | Invocation Rate | Description |
|----------|-----------------|-------------|
| `none` | 60-70% | Let Claude decide |
| `simple` | 70-75% | Text-based reminder |
| `forced` | 80-84% | 3-step evaluation process |

### Custom Hooks

Hook script receives prompt on stdin, outputs modified prompt to stdout:

```bash
#!/bin/bash
PROMPT=$(cat)
cat << EOF
[Custom instructions here]
---
$PROMPT
EOF
```

---

## JSON Output Format

```json
{
  "skills": [
    {
      "name": "my-skill",
      "path": "./my-skill",
      "tests": [
        {
          "id": "test-001",
          "iterations": 5,
          "passed": 4,
          "failed": 1,
          "pass_rate": 80.0,
          "verdict": "Pass",
          "failures": ["iteration 3: assertion 'check-output' failed"],
          "golden_failures": ["iteration 2: 'best-practice' not met"]
        }
      ],
      "verdict": "Pass"
    }
  ],
  "summary": {
    "total_skills": 1,
    "passed_skills": 1,
    "failed_skills": 0,
    "total_tests": 1,
    "passed_tests": 1,
    "failed_tests": 0
  }
}
```

---

## Error Log Files

When tests fail, detailed logs are automatically saved to `.skill-test-logs/` in each skill directory.

### Log File Location

```
my-skill/
  .skill-test-logs/
    20260130-143021-123-0000.json   # YYYYMMDD-HHMMSS-mmm-NNNN.json
    20260130-143045-456-0001.json
```

### Log File Format

Error logs use the same JSON schema as `--format json` output:

```json
{
  "timestamp": "2026-01-30T14:30:21.123Z",
  "skills": [{
    "skill_name": "my-skill",
    "skill_path": "./my-skill",
    "tests": [{
      "name": "test-001",
      "prompt": "...",
      "iterations": [...],
      "summary": {...}
    }],
    "verdict": "Fail",
    "error": null
  }],
  "summary": {...}
}
```

### Controlling Error Logs

```bash
# Disable error log output
skill-test --no-error-log

# Add to .gitignore (recommended)
echo ".skill-test-logs/" >> .gitignore
```

---

## CI/CD Integration

### GitHub Actions

```yaml
- name: Run skill tests
  env:
    ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}
  run: |
    skill-test ./my-skill --threshold 80
```

### JSON Output Processing

```bash
# JSON output
skill-test ./my-skill --format json > results.json

# Parse with jq
cat results.json | jq '.skills[] | select(.verdict == "Fail")'
```

---

## Best Practices

1. **Minimal required assertions**: Only include truly required conditions
2. **Use golden_assertions for quality**: "Nice to have" conditions go in golden_assertions
3. **Tolerant regex patterns**: Allow for whitespace/indentation variations
4. **Clear assertion IDs**: Use meaningful names for easier debugging
5. **Start loose, then tighten**: Begin with relaxed assertions, add more as stability improves

---

## Resources

### Official Documentation

- [Skills](https://code.claude.com/docs/en/skills)
- [Hooks](https://code.claude.com/docs/en/hooks)
- [GitHub Actions](https://code.claude.com/docs/en/github-actions)
