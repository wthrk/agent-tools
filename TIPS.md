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

100行以上のファイルには先頭に目次を追加:

```markdown
# API Reference

## Contents
- Authentication
- Core methods
- Error handling
- Examples
```

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
