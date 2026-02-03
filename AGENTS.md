# AGENTS.md

## Project Overview

Claude Codeスキル管理CLI。スキルのインストール、同期、テストを提供する。

- **Language**: Rust 1.85 (Edition 2024)
- **Build**: Cargo workspace
- **VCS**: jj (Jujutsu) - gitコマンド禁止

## Installation Layout

```
~/.agent-tools/           # インストール先
├── bin/                  # 実行ファイル
├── skills/               # グローバルスキル
└── config.yaml           # 設定

~/.claude/skills/         # Claude Code参照先（シンボリックリンク）
```

`agent-tools sync` で config.yaml に基づき ~/.claude/skills/ にリンクを作成。

## Directory Structure

```
crates/
├── agent-tools/          # メインCLI
├── skill-test/           # スキルテストランナー
└── skill-test-core/      # テストコアライブラリ
skills/                   # 同梱スキル
global/                   # グローバル設定テンプレート
xtask/                    # ビルドタスク
```

## Development Commands

```bash
cargo build --release     # ビルド
cargo xtask install       # インストール
agent-tools sync          # スキル同期
```

## Testing

```bash
cargo test                # Rustユニットテスト
skill-test                # スキル動作検証（Claude CLI必須）
```

## Code Style

- `unwrap()`, `expect()`, `panic!()` 禁止 (deny)
- `unsafe` 禁止 (deny)
- Clippy: all, pedantic, nursery, cargo を warn
- エラーは Result で伝播

## Boundaries

### Always Do
- jj コマンドを使用
- エラーは Result で伝播
- Clippy警告を解消

### Never Do
- git コマンド使用禁止
- unwrap/expect/panic 禁止
- unsafe 禁止
