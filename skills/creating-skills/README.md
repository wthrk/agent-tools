# スキル作成

## 目次

- [概要](#概要)
- [使用条件](#使用条件)
- [ワークフロー](#ワークフロー)
  - [モードA: 新規スキル作成](#モードa-新規スキル作成)
  - [モードB: 既存スキル修正](#モードb-既存スキル修正)
  - [モードC: スキル検証](#モードc-スキル検証)
- [ルール](#ルール)
  - [name](#name)
  - [description](#description)
  - [オプションフィールド](#オプションフィールド)
  - [構造](#構造)
  - [Progressive Disclosure](#progressive-disclosure)
  - [ディレクトリ構造](#ディレクトリ構造)
  - [スクリプト設計](#スクリプト設計)
  - [コンテンツ原則](#コンテンツ原則)
  - [多言語対応](#多言語対応)
  - [静的解析](#静的解析)
- [ヒント](#ヒント)

## 概要

Claude Codeスキルを作成・メンテナンスします。作成、修正、検証の3つのモードをサポート。

## 使用条件

- 新しいスキルをゼロから作成する場合
- 既存のスキルを修正・改善する場合
- デプロイ前にスキルを検証する場合
- 検証エラーや警告を修正する場合

## ワークフロー

### モードA: 新規スキル作成

1. SKILL.md, README.md, AGENTS.mdを含むスキルディレクトリを作成
2. 下記ルールに従ってSKILL.mdを編集
3. README.mdを日本語に翻訳
4. `agent-tools skill validate <path>` で検証
5. エラーや警告を修正

### モードB: 既存スキル修正

1. スキルディレクトリ内の全ファイルを読む
2. 下記ルールに従って修正を適用
3. README.mdをSKILL.mdの変更と同期
4. `agent-tools skill validate <path>` で検証

### モードC: スキル検証

1. `agent-tools skill validate <path>` を実行
2. 続行前にエラーを修正
3. 下記ルールと照合
4. README.mdがSKILL.mdと一致しているか確認
5. 問題を報告（位置と修正案）

## ルール

### name

- 動名詞形式: `processing-pdfs`, `analyzing-data`
- 代替: `pdf-processing`, `process-pdfs`
- 避ける: `helper`, `utils`, `documents`
- 正規表現: `^[a-z0-9][a-z0-9-]*[a-z0-9]$`、最大64文字

### description

形式: `[機能説明]. Use when [トリガー条件].`

- 三人称: "Creates...", "Analyzes..."
- 100-300文字推奨、最大1024
- `<` `>` 禁止

良い例: `Scans Algorand smart contracts for 11 common vulnerabilities. Use when auditing Algorand projects.`

悪い例: `For async testing`（曖昧すぎ）、`I can help you...`（一人称）

### オプションフィールド

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

### 構造

必須セクション: Overview, When to Use, The Process, Tips（Contents, Rules等の追加可）

| 項目 | 推奨 | 最大 |
|------|------|------|
| 行数 | < 500 | - |
| 語数 | < 5,000 | 10,000 |

100行超のファイルには先頭に目次が必要（サブセクションも含める）。

### Progressive Disclosure

| レベル | 内容 | ロード条件 |
|--------|------|-----------|
| 1 | メタデータ | 常時 |
| 2 | SKILL.md本文 | トリガー時 |
| 3 | references/scripts/assets | 必要時のみ（保証なし） |

**設計原則:** Level 3は読まれない可能性がある。
- 確実に適用すべきルール → Level 2（SKILL.md本文）
- 簡潔な例はLevel 2可、詳細な例 → Level 3（references/）
- Level 3が読まれなくてもスキルが正しく動作するよう設計

### ディレクトリ構造

```
skill-name/
├── SKILL.md                # 必須
├── README.md               # 日本語（多言語対応）
├── AGENTS.md               # 同期指示
├── references/             # オプション、1レベル深度のみ
├── scripts/
└── assets/
```

非推奨: INSTALLATION_GUIDE.md, QUICK_REFERENCE.md, CHANGELOG.md

### スクリプト設計

- 単一責任
- 明示的なエラーハンドリング
- JSON/markdown出力
- ログに機密情報を含めない
- 実行前に--helpで使用方法を確認

### コンテンツ原則

コア:
1. 簡潔さ（Claudeは賢い）
2. 一貫した用語
3. 具体的な例

避ける:
- 時間依存情報（"2024年時点で..."）
- 長いオプションリスト（簡潔な代替案はOK）
- Windowsパス
- 一人称（"I can help..."）

### 多言語対応

- SKILL.md: 英語
- README.md: 日本語翻訳
- AGENTS.md: 同期指示

### 静的解析

エラーチェック（終了コード1）:
- SKILL.md存在、frontmatter形式、YAML解析
- 必須フィールド（name, description）
- name形式/長さ、description禁止文字/長さ

警告チェック（終了コード2）:
- 行数 > 500、語数 > 5000
- 禁止ファイル、参照深度 > 1
- 100行超ファイルに目次なし

## ヒント

- よくある間違い: descriptionの一人称、"Use when"の欠落、曖昧な名前
- 重要情報はファイル先頭に配置（Claudeは部分的にプレビューする可能性）
- references/は1レベル深度のみ（ネストされた参照は不完全に読まれる）
