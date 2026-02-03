# AGENTS.md 管理スキル

## 概要

AGENTS.mdファイルを作成・管理するスキル。AIコーディングエージェント向けに広く使われている標準フォーマットに対応。

**主な特徴:**
- 標準Markdown、必須フィールドなし
- モノレポではネストしたAGENTS.mdをサポート（最も近いファイルが優先）
- 32KiBのサイズ制限（Codexデフォルト）
- Codex、Cursor、Copilot等の複数エージェントが同じファイルを参照

## 使用場面

- 新規プロジェクトでAIエージェント連携をセットアップ
- プロジェクト固有の規約やコマンドをドキュメント化
- 古くなったプロジェクト情報を更新
- AGENTS.mdの構造と完全性を検証

## 使い方

### 表示または作成提案

```
/agents-md
```

AGENTS.mdが存在すれば内容を表示、存在しなければ作成を提案。

### 新規作成

```
/agents-md create
```

1. プロジェクト情報を自動検出
2. 検出結果を確認
3. 不足情報を質問
4. テンプレートに基づいて生成
5. プレビュー表示後に書き込み

### セクション更新

```
/agents-md update <section>
```

有効なセクション: overview, stack, structure, commands, style, testing, boundaries, security

### 検証

```
/agents-md validate
```

チェック項目:
- 構造: 必要セクションの存在
- 行数: 150行超で警告
- コマンド: コードブロックの存在
- シークレット: 認証情報の検出
- 完全性: 6つのコアセクションに基づくスコア

## テンプレート

```markdown
# Agent Instructions

## Project Overview
[1-2文: プロジェクトの概要と主要技術]

## Technology Stack
- Language: [言語とバージョン]
- Framework: [フレームワークとバージョン]
- Package Manager: [パッケージマネージャー]

## Directory Structure
```
[ツリー形式、主要ディレクトリのみ]
```

## Development Commands
```bash
# 依存関係インストール
[インストールコマンド]

# テスト実行
[テストコマンド]

# ビルド
[ビルドコマンド]

# 開発サーバー起動
[開発サーバーコマンド]
```

## Code Style Guidelines
- [具体的なルール1]
- [具体的なルール2]

## Testing
- Framework: [テストフレームワーク]
- Coverage: [カバレッジ要件]

## Boundaries

### Always Do
- [必ず行うこと]

### Never Do
- シークレットや認証情報をコミットしない
- [保護対象ファイル]を変更しない

## Security Considerations
- [セキュリティ注意事項]
```

## 6つのコアセクション

1. **Commands** - ビルド、テスト、実行コマンド
2. **Testing** - フレームワーク、カバレッジ、特別なセットアップ
3. **Directory Structure** - ディレクトリ構成
4. **Code Style** - フォーマット、パターン、リンター
5. **Git Workflow** - ブランチ戦略、コミット規約
6. **Boundaries** - 触ってはいけないもの

## ベストプラクティス

- 150行以下を目標
- コマンドは実行可能な形式でコードブロックに記述
- 早い段階でコマンドを配置（フラグ付き）
- 説明よりコード例を優先
- 重複を避け外部リソースにリンク

## エラーリカバリ

- 誤上書き: `jj undo` または `git checkout AGENTS.md`
- 作成失敗: `jj op restore <id>` または `git restore AGENTS.md`
