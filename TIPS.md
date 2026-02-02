# Claude Code Agent Skill 開発 Tips

公式ドキュメント: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

## 基本原則

### 簡潔さが鍵

コンテキストウィンドウは共有リソース。Claude は既に賢いので、不要な説明は省く。

```markdown
# ✅ 良い例（約50トークン）
## PDF テキスト抽出

pdfplumber を使用:

```python
import pdfplumber
with pdfplumber.open("file.pdf") as pdf:
    text = pdf.pages[0].extract_text()
```

# ❌ 悪い例（約150トークン）
## PDF テキスト抽出

PDF（Portable Document Format）は...（長い説明）
```

### 自由度の設定

| 自由度 | 使用場面 | 例 |
|--------|----------|-----|
| 高 | 複数アプローチが有効 | コードレビュー手順 |
| 中 | 推奨パターンあり | テンプレート付きスクリプト |
| 低 | 操作が脆弱/一貫性重要 | DB マイグレーション |

---

## SKILL.md の構造

### frontmatter 必須フィールド

```yaml
---
name: processing-pdfs          # 64文字以内、小文字-数字-ハイフンのみ
description: Extracts text...  # 1024文字以内、第三者視点
---
```

**name の制約:**
- gerund 形式推奨（`processing-pdfs`, `analyzing-data`）
- 小文字、数字、ハイフンのみ
- XML タグ禁止
- "anthropic", "claude" 禁止

**description のルール:**
- **必ず第三者視点**で書く
- 何をするか + いつ使うか を明記
- 具体的なキーワードを含める

```yaml
# ✅ 良い例
description: Extracts text and tables from PDF files. Use when working with PDF files or document extraction.

# ❌ 悪い例
description: I can help you process PDFs  # 一人称禁止
description: Helps with documents          # 曖昧すぎ
```

### オプションフィールド

| フィールド | 説明 |
|-----------|------|
| `allowed-tools` | 使用可能ツール |
| `user-invocable` | `false` で /メニュー非表示 |
| `disable-model-invocation` | `true` で手動のみ |
| `argument-hint` | 引数ヒント |

---

## ファイル構成

### 基本構造

```
my-skill/
├── SKILL.md           # 必須：500行以下
├── reference.md       # 詳細ドキュメント
├── examples.md        # 使用例
└── scripts/           # ユーティリティスクリプト
```

### Progressive Disclosure

SKILL.md は目次として機能。詳細は別ファイルに分割。

| レベル | 内容 | ロード条件 |
|--------|------|-----------|
| 1 | メタデータ（name, description） | 常時 |
| 2 | SKILL.md 本文 | トリガー時 |
| 3 | references/scripts/assets | 必要時のみ（保証なし） |

**設計原則:** Level 3 は読まれない可能性がある。
- 確実に適用すべきルール → Level 2（SKILL.md本文）
- 詳細・例・背景 → Level 3（references/）
- 「〇〇を読め」という指示は無視される可能性あり
- Level 3 が読まれなくてもスキルが正しく動作するよう設計

```markdown
# SKILL.md

## Quick start
[基本的な使い方]

## Advanced features
**Form filling**: See [FORMS.md](FORMS.md)
**API reference**: See [REFERENCE.md](REFERENCE.md)
```

**重要:** 参照は **1レベルのみ**。深いネストは避ける。

```markdown
# ❌ 悪い例（深すぎ）
SKILL.md → advanced.md → details.md

# ✅ 良い例（1レベル）
SKILL.md → advanced.md
SKILL.md → reference.md
SKILL.md → examples.md
```

### 長いファイルには目次

100行以上のファイルには先頭に目次を追加。**サブセクションも含める**:

```markdown
# API Reference

## Contents
- Authentication
- Core methods
  - create
  - read
  - update
  - delete
- Error handling
- Examples
```

トップレベルのみの目次では部分読みされた際に詳細が見落とされる。

---

## ワークフローパターン

### チェックリスト形式

```markdown
## PDF フォーム入力ワークフロー

進捗をチェック:

```
- [ ] Step 1: フォーム解析
- [ ] Step 2: フィールドマッピング
- [ ] Step 3: バリデーション
- [ ] Step 4: フォーム入力
- [ ] Step 5: 出力検証
```

**Step 1: フォーム解析**
Run: `python scripts/analyze_form.py input.pdf`
...
```

### フィードバックループ

バリデーター → 修正 → 繰り返し

```markdown
## 編集プロセス

1. `word/document.xml` を編集
2. **即座にバリデート**: `python scripts/validate.py`
3. 失敗したら修正して再バリデート
4. **パスしたら次へ**
5. リビルド: `python scripts/pack.py`
```

---

## セキュリティ

### 最小権限の原則

```yaml
---
name: reading-files
description: Reads files without modification
allowed-tools: Read, Grep, Glob
---
```

### hooks で危険操作をガード

```yaml
---
name: modifying-code
hooks:
  PreToolUse:
    - matcher: "Edit|Write|Bash"
      hooks:
        - type: command
          command: ./scripts/validate.sh
---
```

---

## スクリプト

### エラーハンドリング

```python
# ✅ 良い例：エラーを処理
def process_file(path):
    try:
        with open(path) as f:
            return f.read()
    except FileNotFoundError:
        print(f"File {path} not found, creating default")
        with open(path, 'w') as f:
            f.write('')
        return ''

# ❌ 悪い例：Claude に丸投げ
def process_file(path):
    return open(path).read()
```

### 定数の文書化

```python
# ✅ 良い例
REQUEST_TIMEOUT = 30  # HTTP requests typically complete within 30 seconds
MAX_RETRIES = 3       # Most failures resolve by second retry

# ❌ 悪い例
TIMEOUT = 47  # Why 47?
```

---

## 評価とイテレーション

### 評価を先に作成

1. スキルなしでタスク実行、失敗を記録
2. 3つのテストシナリオを作成
3. ベースライン測定
4. 最小限の指示を作成
5. イテレーション

### Claude と一緒に開発

1. Claude A でスキル設計
2. Claude B（新規インスタンス）でテスト
3. 問題を Claude A にフィードバック
4. 改善してテスト繰り返し

---

## アンチパターン

| 問題 | 解決策 |
|------|--------|
| Windows パス (`\`) | forward slash (`/`) のみ使用 |
| 選択肢が多すぎ | デフォルトを1つ提供 |
| 時間依存の情報 | "old patterns" セクションに移動 |
| 用語の不一致 | 1つの用語を一貫して使用 |
| 深いネスト参照 | 1レベルのみ |
| 一人称使用 | 三人称で記述（"Creates..." など） |
| 曖昧な description | 具体的なキーワードとトリガー条件を含める |

---

## Claude の読み込み挙動

### 観測された傾向

- 大きなファイルを `head -100` 等でプレビューする傾向がある
- ネストされた参照は不完全に読まれる傾向がある

### 対策

| 問題 | 対策 |
|------|------|
| 部分読み込み | 100行超は先頭に**目次必須** |
| ネスト参照 | **1レベル深度のみ** |
| 重要情報見落とし | **ファイル先頭に配置** |

---

## 静的解析

### コマンド

```bash
agent-tools skill validate ./my-skill/
```

### 検証項目

**エラー（終了コード 1）:**
- SKILL.md 存在
- Frontmatter 形式（`---` 区切り）
- YAML 解析
- 必須フィールド（name, description）
- name 形式・長さ（64文字以内）
- description 禁止文字（`<` `>`）・長さ（1024文字以内）
- 許可されていないキー

**警告（終了コード 2）:**
- 行数超過（500行超）
- 語数超過（5000語超）
- 禁止ファイル存在（CHANGELOG.md 等）
- 参照深度超過（1レベル超）
- 目次なし（100行超のファイル）

### 終了コード

| コード | 意味 |
|--------|------|
| 0 | 成功 |
| 1 | エラーあり |
| 2 | 警告のみ（`--strict` で 1） |

---

## 多言語対応

### ファイル構造

```
skill-name/
├── SKILL.md    # 英語
├── README.md   # 日本語（SKILL.md の翻訳）
└── AGENTS.md   # 同期指示
```

### AGENTS.md の目的

Claude にスキル全体と README.md の同期を指示:

```markdown
# Agent Instructions

README.md is the Japanese explanation of this skill.
When updating SKILL.md or any related files, also update README.md to keep them in sync.
```

### README.md の役割

- SKILL.md と同等の構造で日本語説明
- セクション: 概要 → 使用条件 → ワークフロー → ヒント
- SKILL.md 更新時は同期必須

---

## チェックリスト

### コア品質
- [ ] description: 第三者視点、具体的キーワード
- [ ] description: 何をするか + いつ使うか
- [ ] name: gerund 形式、小文字-ハイフン
- [ ] SKILL.md: 500行以下
- [ ] 詳細は別ファイルに分割
- [ ] 参照は1レベルのみ
- [ ] 一貫した用語
- [ ] forward slash のみ

### スクリプト
- [ ] エラーハンドリング明示
- [ ] 定数に説明コメント
- [ ] パッケージ依存を記載

### テスト
- [ ] 3つ以上の評価シナリオ
- [ ] Haiku, Sonnet, Opus でテスト
- [ ] 実際の使用シナリオでテスト

---

## 参考リンク

- [Skills Overview](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/overview)
- [Best Practices](https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices)
- [Claude Code Skills](https://code.claude.com/docs/en/skills)
