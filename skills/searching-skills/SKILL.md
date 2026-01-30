---
name: searching-skills
description: Searches the web for Claude Code agent skills and evaluates them. Use when user wants to find, discover, search for, or explore Claude Code skills, agent skills, or SKILL.md files.
allowed-tools: WebSearch, WebFetch, AskUserQuestion, Task
user-invocable: true
argument-hint: [keywords]
---

# EXECUTE IMMEDIATELY - DO NOT EXPLAIN

When this skill is invoked, you MUST immediately start searching. Do NOT describe what this skill does. EXECUTE the steps below.

## STEP 1: SEARCH NOW

Execute these WebSearch queries immediately:

1. `WebSearch: Claude Code skills $ARGUMENTS`
2. `WebSearch: site:github.com Claude Code SKILL.md $ARGUMENTS`

## STEP 2: EVALUATE RESULTS

For the top 3-5 results, evaluate each on these criteria (1-5 points each):
- relevance, completeness, reliability, usability, maintenance, best_practices, security
- Total: sum of all scores out of 35

## STEP 3: OUTPUT IN THIS EXACT FORMAT

After evaluating, output EXACTLY like this:

Found N skills (sorted by score):

1. ⭐ 28/35 **skill-name-here** - One line description
   Source: https://actual-url-here | Updated: 2024-01-15
   🔒 Security: OK

2. ⭐ 24/35 **another-skill** - Another description
   Source: https://another-url | Updated: 2024-01-10
   ⚠️ Security: WARNING - reason

(Include Purpose:, Configuration:, How it works:, Usage: sections for detail)

## CRITICAL RULES

- Start searching IMMEDIATELY when invoked
- Include "site:" in search queries
- Use X/35 score format
- Include 🔒/⚠️/🚨 Security: indicator
- Include Source: URL
- Use **bold** for skill names
