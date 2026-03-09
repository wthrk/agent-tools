# agent-tools

Claude Code スキル管理CLI。グローバルスキルの管理、プロジェクトへのインストールを行います。

> **対応OS**: macOS / Linux のみ（Windowsは非対応）

## 概要

agent-toolsは以下の機能を提供します:

- **グローバルスキル管理**: `~/.agent-tools/skills/` でスキルを一元管理
- **プロジェクトへのインストール**: グローバルスキルをプロジェクトにコピー
- **自動デプロイ**: `config.yaml` で指定したスキルを `~/.claude/skills/` にリンク
- **Claude MCP同期**: `claude_mcp_servers` を同期（不要になった managed MCP は対話確認で削除）
- **Codex設定生成**: `codex/config.toml` と `~/.codex/config.local.toml` をマージして `~/.codex/config.toml` を生成
- **Codexサブエージェント同期**: `~/.agent-tools/codex/agents/` を `~/.codex/agents/` に同期
- **スキル検証**: SKILL.md のフロントマター・構造を検証

## 前提条件

- Rust (cargo)
- Git

## インストール

```bash
git clone <repository-url> ~/.agent-tools
cd ~/.agent-tools
cargo build --release
cargo xtask install
agent-tools init
```

`init` 実行後、表示される指示に従ってPATHを設定してください:

```bash
# ~/.bashrc または ~/.zshrc に追加
export PATH="$HOME/.agent-tools/bin:$PATH"
```

## クイックスタート

```bash
# 新規スキル作成
agent-tools skill new my-skill

# スキル一覧確認
agent-tools skill list

# プロジェクトにインストール
cd my-project
agent-tools skill install my-skill
```

## コマンドリファレンス

### トップレベルコマンド

| コマンド | 説明 |
|----------|------|
| `init` | 初期化（ディレクトリ作成、PATH設定指示を表示） |
| `status` | 現在の状態表示（リンク、設定検証） |
| `sync [--dry-run] [--prune]` | config.yamlに基づく同期 |
| `link <name>` | スキルを `~/.claude/skills/` にリンク |
| `unlink <name>` | スキルをアンリンク |
| `build` | ビルド＆インストール |
| `update` | アップデート（git pull && cargo build） |
| `cleanup` | 古いバックアップ削除 |
| `runpod up <profile>` | `templates/claude/<profile>/runpod.yaml` に基づき Pod を作成・起動 |

### skill サブコマンド

| コマンド | 説明 | オプション |
|----------|------|----------|
| `skill new <name>` | 新規スキル作成 | `-y`, `--no-auto-deploy` |
| `skill list` | グローバルスキル一覧 | - |
| `skill install <name>` | プロジェクトにインストール | `--project <path>` |
| `skill update [name]` | スキル更新 | `--all`, `--force`, `--project` |
| `skill remove <name>` | スキル削除 | `--project` |
| `skill installed` | インストール済み一覧 | `--project` |
| `skill diff <name>` | 差分表示 | `--project` |
| `skill validate [path]` | 検証 | `--strict` |

## 設定ファイル

### config.yaml

`~/.agent-tools/config.yaml`:

```yaml
config_version: 1
auto_deploy_skills:
  - my-skill
  - another-skill
manage_settings: false
manage_plugins: false
```

| 項目 | 説明 |
|------|------|
| `config_version` | 設定バージョン（現在: 1） |
| `auto_deploy_skills` | `~/.claude/skills/` に自動リンクするスキル名 |
| `manage_settings` | settings.jsonを管理するか |
| `manage_plugins` | plugins/を管理するか |
| `manage_claude_md` | `~/.claude/CLAUDE.md` を管理するか |
| `manage_hooks` | `~/.claude/hooks/` を管理するか |
| `manage_codex_config` | `~/.codex/config.toml` を生成管理するか（base + local マージ） |
| `claude_mcp_servers` | Claude MCP サーバー定義（同期対象） |

## ディレクトリ構造

### agent-tools ホーム (`~/.agent-tools/`)

```
~/.agent-tools/
├── bin/           # 実行ファイル (agent-tools)
├── skills/        # グローバルスキル
│   └── my-skill/
│       ├── SKILL.md
│       ├── README.md
│       └── AGENTS.md
├── codex/
│   ├── config.toml    # Codex共通base設定
│   └── agents/        # Codexサブエージェント設定
├── backups/       # バックアップ
├── config.yaml    # 設定
├── settings.json  # (任意) manage_settings: true時
└── plugins/       # (任意) manage_plugins: true時
```

### Codex ローカル構造（生成先）

```
~/.codex/
├── config.toml         # 生成物（base + config.local.toml）
├── config.local.toml   # 端末固有設定（任意）
└── agents/             # codex/agents から同期
```

`config_file` は相対パス（例: `agents/worker.toml`）で管理してください。

### RunPod プロファイル設定

RunPod の設定は profile に紐づけて `templates/claude/<profile>/runpod.yaml` に配置します。

例:

```yaml
name: runpod-llm
template_id: runpod-torch-v21
gpu_id: NVIDIA RTX 4090
cloud_type: SECURE
compute_type: GPU
gpu_count: 1
container_disk_in_gb: 20
ports:
  - 8000/http
start_after_create: true
```

実行:

```bash
agent-tools runpod up runpod
```

Dockerでモデル起動を行う場合は `templates/claude/<profile>/docker/` の
`Dockerfile` と `entrypoint.sh` を使ってイメージを作成・pushし、
`runpod.yaml` の `image` に設定してください。

### プロジェクト構造

```
project/
└── .claude/
    └── skills/
        └── my-skill/
            ├── SKILL.md
            ├── README.md
            ├── AGENTS.md
            └── .skill-meta.yaml
```

## ファイル形式仕様

### SKILL.md フロントマター

```yaml
---
name: my-skill              # 必須: 最大64文字, kebab-case
description: ...            # 必須: 最大1024文字
license: MIT                # 任意
allowed-tools: []           # 任意: ツール制限
metadata: {}                # 任意
user-invocable: true        # 任意: /menuに表示
disable-model-invocation: false  # 任意
argument-hint: <arg>        # 任意
---
```

### .skill-meta.yaml

プロジェクトにインストールされたスキルのメタデータ:

```yaml
source: /Users/xxx/.agent-tools/skills/my-skill
tree_hash: abc123...
installed_at: 2026-01-30T12:00:00Z
updated_at: 2026-01-30T12:00:00Z
```

## バリデーションルール

`skill validate` で検証されるルール:

### name

- 正規表現: `^[a-z0-9]([a-z0-9-]*[a-z0-9])?$`
- 最大64文字
- 連続ハイフン禁止
- 先頭/末尾は英数字

### description

- 最大1024文字
- `<` `>` 禁止

### ファイルサイズ（警告）

- SKILL.md: 500行以下推奨
- ワード数: 5000語以下推奨
- 100行超えファイル: 目次推奨

### 禁止ファイル（警告）

- CHANGELOG.md
- INSTALLATION_GUIDE.md
- QUICK_REFERENCE.md

### その他

- 未知のフロントマターキーはエラー
- references/内のファイルが他の.mdファイルを参照している場合は警告（参照深度 > 1）

### 終了コード

| コード | 意味 |
|--------|------|
| 0 | 成功（エラー・警告なし） |
| 1 | エラーあり（`--strict`時は警告含む） |
| 2 | 警告のみ |

## 環境変数

| 変数 | 説明 | デフォルト |
|------|------|----------|
| `AGENT_TOOLS_HOME` | ホームディレクトリ | `~/.agent-tools` |
| `CLAUDE_HOME` | Claudeホーム | `~/.claude` |
| `CODEX_HOME` | Codexホーム | `~/.codex` |

## トラブルシューティング

### `SKILL.md not found`

スキルディレクトリにSKILL.mdが存在するか確認してください。

### `Name must match pattern...`

スキル名は小文字英数字とハイフンのみ使用可能です。先頭と末尾は英数字である必要があります。

### `sync` で MCP 削除確認が出る

`claude_mcp_servers` から削除した managed MCP は、`sync` 実行時に対話確認（`y/N`）のうえで削除されます。  
非対話セッションでは安全のため削除せずスキップされます。

## 開発

```bash
# ビルド
cargo build --release

# インストール
cargo xtask install

# テスト
cargo test
```

## ライセンス

MIT

## 参考リンク

- [Claude Code Skills](https://code.claude.com/docs/en/skills)
- [Claude Code Plugins](https://code.claude.com/docs/en/plugins)
- [Claude Code Hooks](https://code.claude.com/docs/en/hooks)
- [Claude Code Best Practices](https://code.claude.com/docs/en/best-practices)
- [Agent Skills Standard](https://agentskills.io)
