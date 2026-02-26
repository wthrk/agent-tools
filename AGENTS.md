# AGENTS.md

## Overview

CLI tool for managing Claude Code skills globally. Centralizes skills under `~/.agent-tools/skills/`, providing auto-deploy via `config.yaml`, installation into projects, and SKILL.md validation.

Rust 1.85, Cargo workspace, jj (no git)

## Structure

```
crates/
└── agent-tools/       # CLI
```

## Commands

```bash
cargo xtask install         # ビルド → ~/.agent-tools/bin/agent-tools
cargo xtask ci              # fmt, check, clippy, deny, test, skill-validate
cargo xtask skill-validate  # skills/配下のスキルを検証
cargo xtask test-all        # ci + docker + integration
```

## Installation

```bash
git clone <repo> ~/.agent-tools && cd ~/.agent-tools
cargo xtask install
export PATH="$HOME/.agent-tools/bin:$PATH"
```

## Quick Checks (single file)

```bash
cargo fmt -- --check crates/agent-tools/src/<file>.rs
cargo clippy -p agent-tools -- -D warnings
cargo test -p agent-tools -- <test_name>
```

## Boundaries

- jj使用（git禁止）
- unwrap/expect/panic/unsafe 禁止
- Result で伝播
