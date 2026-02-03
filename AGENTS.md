# AGENTS.md

## Overview

Claude Codeスキル管理CLI。Rust 1.85, Cargo workspace, jj (git禁止)

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

## Boundaries

- jj使用（git禁止）
- unwrap/expect/panic/unsafe 禁止
- Result で伝播
