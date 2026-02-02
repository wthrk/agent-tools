# agent-tools

Claude Code スキル管理CLI。グローバルスキルの管理、プロジェクトへのインストール、スキルテストの実行を行います。

> **対応OS**: macOS / Linux のみ（Windowsは非対応）

## 概要

agent-toolsは以下の機能を提供します:

- **グローバルスキル管理**: `~/.agent-tools/skills/` でスキルを一元管理
- **プロジェクトへのインストール**: グローバルスキルをプロジェクトにコピー
- **自動デプロイ**: `config.yaml` で指定したスキルを `~/.claude/skills/` にリンク
- **スキル検証**: SKILL.md のフロントマター・構造を検証
- **スキルテスト**: `skill-test` コマンドでスキルの動作をテスト

## 前提条件

- Rust (cargo)
- Git
- Claude CLI（skill-test使用時）

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

# スキルテスト実行
skill-test
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

## ディレクトリ構造

### agent-tools ホーム (`~/.agent-tools/`)

```
~/.agent-tools/
├── bin/           # 実行ファイル (agent-tools, skill-test)
├── skills/        # グローバルスキル
│   └── my-skill/
│       ├── SKILL.md
│       ├── README.md
│       └── AGENTS.md
├── backups/       # バックアップ
├── config.yaml    # 設定
├── settings.json  # (任意) manage_settings: true時
└── plugins/       # (任意) manage_plugins: true時
```

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

---

## skill-test コマンド

スキルの動作を検証するテストランナー。

> **前提**: Claude CLIがPATHに存在し、認証済みである必要があります。

### 使用方法

```bash
skill-test [SKILL_DIR...] [OPTIONS]
```

### オプション

| オプション | 説明 | デフォルト |
|-----------|------|----------|
| `--iterations <N>` | 繰り返し回数 | 10 |
| `--threshold <N>` | 合格閾値(%) | 80 |
| `--model <MODEL>` | モデル | claude-sonnet-4-20250514 |
| `--timeout <MS>` | タイムアウト(ms) | 60000 |
| `--hook <TYPE>` | フック戦略 (none/simple/forced/custom) | simple |
| `--hook-path <PATH>` | カスタムフックパス（hook=custom時必須） | - |
| `--strict` | 厳格モード | false |
| `--verbose` / `-v` | 詳細出力 | false |
| `--format <FMT>` | 出力形式 (table/json) | table |
| `--filter <PATTERN>` | テストフィルタ | - |
| `--parallel <N>` | 並列数（0=逐次） | CPU数 |
| `--no-color` | 色無効化 | false |
| `--no-error-log` | エラーログ無効化 | false |

### skill-test.config.yaml

スキルディレクトリに配置してデフォルト設定を上書き:

```yaml
model: claude-sonnet-4-20250514
timeout: 60000
iterations: 10
threshold: 80
hook: simple
hook-path: ./my-hook.sh      # hook: custom時のみ
test-patterns:
  - "skill-tests/**/test-*.yaml"
  - "skill-tests/**/test-*.yml"
  - "skill-tests/**/*.spec.yaml"
  - "skill-tests/**/*.spec.yml"
exclude-patterns:
  - "node_modules/"
strict: false
```

## テストファイル形式

### シナリオ形式（推奨）

```yaml
desc: "テストファイル説明"

assertions:
  check-greeting:
    desc: "挨拶が含まれていること"
    type: contains
    pattern: "Hello"
    expect: present

scenarios:
  greeting-test:
    desc: "挨拶テスト"
    prompt: "Say hello"
    iterations: 5  # オプション: 上書き
    assertions:
      - check-greeting           # 名前参照
      - type: contains           # インライン定義
        id: inline-check
        pattern: "World"
        expect: present
    golden_assertions:           # 情報用（Pass/Failに影響しない）
      - check-greeting
```

### 配列形式（レガシー）

```yaml
- id: test-001
  prompt: "Say hello"
  assertions:
    - id: check
      type: contains
      pattern: "Hello"
      expect: present
```

## アサーション型

### contains

文字列の存在/不在を確認:

```yaml
type: contains
pattern: "検索文字列"
expect: present|absent
```

### regex

正規表現でマッチ:

```yaml
type: regex
pattern: "\\d+\\."
expect: present|absent
```

### line_count

行数の範囲を確認（少なくとも一方必須）:

```yaml
type: line_count
min: 5
max: 20
```

### exec

コードを実行して結果を検証:

```yaml
type: exec
command: "node|python3|bash"
language: "javascript"
timeout_ms: 10000
expect: "exit_code:0"
# または
expect:
  output_contains: "expected text"
```

### tool_called

特定ツールが呼ばれたかを確認:

```yaml
type: tool_called
pattern: "Skill"
expect: present|absent
```

### llm_eval

LLMで出力を評価:

```yaml
type: llm_eval
pattern: "出力が質問に正しく回答しているか? {{output}}"
expect: pass|fail
timeout_ms: 60000
json_schema:  # 任意
  type: object
  required: [result, reason]
  properties:
    result: {type: boolean}
    reason: {type: string}
```

`{{output}}` はテスト出力で置換されます。

## 環境変数

| 変数 | 説明 | デフォルト |
|------|------|----------|
| `AGENT_TOOLS_HOME` | ホームディレクトリ | `~/.agent-tools` |
| `CLAUDE_HOME` | Claudeホーム | `~/.claude` |

## トラブルシューティング

### `SKILL.md not found`

スキルディレクトリにSKILL.mdが存在するか確認してください。

### `Name must match pattern...`

スキル名は小文字英数字とハイフンのみ使用可能です。先頭と末尾は英数字である必要があります。

### `hook-path is required when hook is 'custom'`

`--hook=custom` を指定した場合、`--hook-path` も必須です。

### テストがタイムアウトする

`--timeout` を増やすか、`skill-test.config.yaml` で `timeout` を設定してください。

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
